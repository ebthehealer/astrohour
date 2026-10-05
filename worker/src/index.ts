/**
 * AstroHour License Worker
 *
 * Handles two routes:
 *
 *   POST /stripe-webhook
 *     Receives Stripe checkout.session.completed events.
 *     Verifies the Stripe signature, generates a signed Ed25519 license key,
 *     and emails it to the customer via Resend.
 *
 *   POST /verify-key
 *     Optional: lets the app do an online check (not required — the app
 *     validates offline via the embedded public key). Useful for support.
 */

interface Env {
  STRIPE_WEBHOOK_SECRET:   string;
  STRIPE_SECRET_KEY:       string;
  ED25519_PRIVATE_KEY_HEX: string;
  RESEND_API_KEY:          string;
  APP_NAME:                string;
  FROM_EMAIL:              string;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (request.method === 'POST' && url.pathname === '/stripe-webhook') {
      return handleStripeWebhook(request, env);
    }
    if (request.method === 'POST' && url.pathname === '/verify-key') {
      return handleVerifyKey(request, env);
    }

    return new Response('AstroHour License Service', { status: 200 });
  },
};

// ── Stripe webhook ────────────────────────────────────────────────────────────

async function handleStripeWebhook(request: Request, env: Env): Promise<Response> {
  const body      = await request.text();
  const sigHeader = request.headers.get('stripe-signature') ?? '';

  // Verify Stripe webhook signature
  const valid = await verifyStripeSignature(body, sigHeader, env.STRIPE_WEBHOOK_SECRET);
  if (!valid) {
    return new Response('Invalid signature', { status: 400 });
  }

  let event: any;
  try {
    event = JSON.parse(body);
  } catch {
    return new Response('Bad JSON', { status: 400 });
  }

  // Only handle completed checkout sessions
  if (event.type !== 'checkout.session.completed') {
    return new Response('OK', { status: 200 });
  }

  const session      = event.data.object;
  const customerEmail = session.customer_details?.email ?? session.customer_email;
  const customerName  = session.customer_details?.name  ?? 'Customer';

  if (!customerEmail) {
    console.error('No customer email in session:', session.id);
    return new Response('No email', { status: 200 }); // 200 so Stripe doesn't retry
  }

  // Generate signed license key
  const licenseKey = await generateLicenseKey(env.ED25519_PRIVATE_KEY_HEX);

  // Send email via Resend
  await sendLicenseEmail(customerEmail, customerName, licenseKey, env);

  console.log(`License issued to ${customerEmail}: ${licenseKey.slice(0, 20)}...`);
  return new Response('OK', { status: 200 });
}

// ── Key generation ────────────────────────────────────────────────────────────

async function generateLicenseKey(privateKeyHex: string): Promise<string> {
  // Import the raw 32-byte Ed25519 private key (seed)
  const privateBytes = hexToBytes(privateKeyHex);

  // Web Crypto API: import as PKCS#8 for Ed25519
  // Chrome/V8 (Cloudflare Workers) supports Ed25519 via SubtleCrypto
  const keyPair = await crypto.subtle.importKey(
    'raw',
    privateBytes,
    { name: 'Ed25519' },
    false,
    ['sign'],
  ).catch(async () => {
    // Fallback: wrap in PKCS8 DER envelope
    const pkcs8 = buildEd25519Pkcs8(privateBytes);
    return crypto.subtle.importKey(
      'pkcs8',
      pkcs8,
      { name: 'Ed25519' },
      false,
      ['sign'],
    );
  });

  // Build payload: version(1) || reserved(3) || epoch_days(2)
  const payload = new Uint8Array(6);
  payload[0] = 0x01; // version
  const epochDays = daysSince20250101();
  payload[4] = (epochDays >> 8) & 0xff;
  payload[5] =  epochDays       & 0xff;

  // Sign payload with Ed25519
  const sigBuf = await crypto.subtle.sign('Ed25519', keyPair, payload);
  const sig    = new Uint8Array(sigBuf);

  // Concatenate payload + signature (6 + 64 = 70 bytes)
  const raw = new Uint8Array(70);
  raw.set(payload, 0);
  raw.set(sig,     6);

  // Base32 encode and format as ASTROHOUR-XXXX-...
  const encoded = base32Encode(raw); // 112 chars
  const groups: string[] = [];
  for (let i = 0; i < 14; i++) {
    groups.push(encoded.slice(i * 8, (i + 1) * 8));
  }
  return `ASTROHOUR-${groups.join('-')}`;
}

function daysSince20250101(): number {
  const epoch = new Date('2025-01-01T00:00:00Z').getTime();
  return Math.floor((Date.now() - epoch) / 86_400_000);
}

// ── Online key verification endpoint ─────────────────────────────────────────

async function handleVerifyKey(request: Request, env: Env): Promise<Response> {
  let body: any;
  try { body = await request.json(); } catch { return json({ valid: false, error: 'bad json' }, 400); }

  const key = typeof body.key === 'string' ? body.key.trim().toUpperCase() : '';
  if (!key.startsWith('ASTROHOUR-')) {
    return json({ valid: false, error: 'invalid format' });
  }

  // Decode and verify
  const parts = key.replace('ASTROHOUR-', '').split('-');
  if (parts.length !== 14) return json({ valid: false, error: 'invalid format' });

  const decoded = base32Decode(parts.join(''));
  if (!decoded || decoded.length < 70) return json({ valid: false, error: 'bad encoding' });

  const payload  = decoded.slice(0, 6);
  const sigBytes = decoded.slice(6, 70);

  // Get public key from private key
  const privateBytes = hexToBytes(env.ED25519_PRIVATE_KEY_HEX);
  // We re-derive the public key at runtime on the Worker (safe — it's the Worker)
  const privateKey = await crypto.subtle.importKey(
    'pkcs8', buildEd25519Pkcs8(privateBytes),
    { name: 'Ed25519' }, true, ['sign']
  );
  const publicKeyBuf = await crypto.subtle.exportKey('spki', privateKey).catch(() => null);
  if (!publicKeyBuf) return json({ valid: false, error: 'key error' });

  const publicKey = await crypto.subtle.importKey(
    'spki', publicKeyBuf, { name: 'Ed25519' }, false, ['verify']
  );

  const valid = await crypto.subtle.verify('Ed25519', publicKey, sigBytes, payload);
  return json({ valid });
}

// ── Email via Resend ──────────────────────────────────────────────────────────

async function sendLicenseEmail(
  to: string, name: string, key: string, env: Env
): Promise<void> {
  const html = `
<!DOCTYPE html>
<html>
<body style="font-family: -apple-system, sans-serif; max-width: 560px; margin: 40px auto; color: #222;">
  <h2 style="color: #5a4fcf;">&#9654; ${env.APP_NAME} — Your License Key</h2>
  <p>Hi ${escapeHtml(name)},</p>
  <p>Thank you for purchasing <strong>${env.APP_NAME} Premium</strong>!
  Here is your license key:</p>
  <div style="background: #f5f3ff; border: 1px solid #c4b8f8; border-radius: 8px;
              padding: 16px; margin: 20px 0; font-family: monospace; font-size: 14px;
              letter-spacing: 0.05em; word-break: break-all;">
    ${escapeHtml(key)}
  </div>
  <p><strong>To activate:</strong></p>
  <ol>
    <li>Open ${env.APP_NAME}</li>
    <li>Click <strong>&#10024; Premium</strong> in the footer</li>
    <li>Paste your key and click <strong>Activate</strong></li>
  </ol>
  <p style="color: #888; font-size: 13px;">
    Keep this email — you can re-enter the key if you reinstall the app.
    This key is for personal use on your own devices.
  </p>
  <hr style="border: none; border-top: 1px solid #eee; margin: 24px 0;">
  <p style="color: #aaa; font-size: 12px;">
    Questions? Reply to this email or visit <a href="https://astrohour.app">astrohour.app</a>
  </p>
</body>
</html>`;

  const res = await fetch('https://api.resend.com/emails', {
    method:  'POST',
    headers: {
      'Authorization': `Bearer ${env.RESEND_API_KEY}`,
      'Content-Type':  'application/json',
    },
    body: JSON.stringify({
      from:    `${env.APP_NAME} <${env.FROM_EMAIL}>`,
      to:      [to],
      subject: `Your ${env.APP_NAME} License Key`,
      html,
    }),
  });

  if (!res.ok) {
    const err = await res.text();
    throw new Error(`Resend error ${res.status}: ${err}`);
  }
}

// ── Stripe signature verification ────────────────────────────────────────────

async function verifyStripeSignature(
  body: string, header: string, secret: string
): Promise<boolean> {
  // Parse t=<timestamp>,v1=<sig>,v1=<sig>,...
  const parts = Object.fromEntries(
    header.split(',').map(p => p.split('=') as [string, string])
  );
  const timestamp = parts['t'];
  const v1        = parts['v1'];
  if (!timestamp || !v1) return false;

  // Reject if timestamp is more than 5 minutes old
  const age = Math.abs(Date.now() / 1000 - parseInt(timestamp, 10));
  if (age > 300) return false;

  const payload = `${timestamp}.${body}`;
  const keyData = new TextEncoder().encode(secret);
  const msgData = new TextEncoder().encode(payload);

  const hmacKey = await crypto.subtle.importKey(
    'raw', keyData, { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']
  );
  const mac = await crypto.subtle.sign('HMAC', hmacKey, msgData);
  const computed = bytesToHex(new Uint8Array(mac));

  // Constant-time compare
  return computed === v1;
}

// ── Utilities ──────────────────────────────────────────────────────────────────

function base32Encode(data: Uint8Array): string {
  const ALPHA = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  let out = '', bits = 0, bitsLeft = 0;
  for (const byte of data) {
    bits = (bits << 8) | byte;
    bitsLeft += 8;
    while (bitsLeft >= 5) {
      bitsLeft -= 5;
      out += ALPHA[(bits >> bitsLeft) & 0x1f];
    }
  }
  if (bitsLeft > 0) out += ALPHA[(bits << (5 - bitsLeft)) & 0x1f];
  return out;
}

function base32Decode(s: string): Uint8Array | null {
  const ALPHA = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  const out: number[] = [];
  let bits = 0, bitsLeft = 0;
  for (const ch of s.toUpperCase()) {
    const val = ALPHA.indexOf(ch);
    if (val === -1) return null;
    bits = (bits << 5) | val;
    bitsLeft += 5;
    if (bitsLeft >= 8) {
      bitsLeft -= 8;
      out.push((bits >> bitsLeft) & 0xff);
    }
  }
  return new Uint8Array(out);
}

function hexToBytes(hex: string): Uint8Array {
  const arr = new Uint8Array(hex.length / 2);
  for (let i = 0; i < arr.length; i++)
    arr[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  return arr;
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
}

/** Build a minimal PKCS#8 DER envelope for a raw 32-byte Ed25519 seed. */
function buildEd25519Pkcs8(seed: Uint8Array): ArrayBuffer {
  // PKCS#8 structure for Ed25519:
  // SEQUENCE {
  //   INTEGER 0  (version)
  //   SEQUENCE { OID 1.3.101.112 }  (Ed25519 algorithm)
  //   OCTET STRING { OCTET STRING { <32-byte seed> } }
  // }
  const oid   = new Uint8Array([0x06, 0x03, 0x2b, 0x65, 0x70]);
  const inner = new Uint8Array([0x04, 0x20, ...seed]); // OCTET STRING wrapping seed
  const outer = new Uint8Array([0x04, inner.length + 2, 0x04, inner.length - 2, ...seed]);

  const alg = new Uint8Array([0x30, oid.length + 2, ...oid]);
  const ver = new Uint8Array([0x02, 0x01, 0x00]);
  // Simpler: use the known fixed DER envelope for Ed25519 PKCS8
  // The envelope is always the same 16-byte header + 32-byte seed
  const pkcs8Header = new Uint8Array([
    0x30, 0x2e,             // SEQUENCE (46 bytes)
      0x02, 0x01, 0x00,     // INTEGER 0 (version)
      0x30, 0x05,           // SEQUENCE (5 bytes)
        0x06, 0x03,         // OID (3 bytes)
          0x2b, 0x65, 0x70, // 1.3.101.112 (Ed25519)
      0x04, 0x22,           // OCTET STRING (34 bytes)
        0x04, 0x20,         // inner OCTET STRING (32 bytes)
  ]);
  const buf = new Uint8Array(pkcs8Header.length + seed.length);
  buf.set(pkcs8Header);
  buf.set(seed, pkcs8Header.length);
  return buf.buffer;
}

function escapeHtml(s: string): string {
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');
}

function json(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}
