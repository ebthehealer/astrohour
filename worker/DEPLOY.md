# Deploying the AstroHour License Worker

## Prerequisites
- Cloudflare account (free tier is fine)
- `npm install -g wrangler` + `wrangler login`
- Stripe account with a product set up ($10 one-time)
- Resend account (free tier: 3,000 emails/month)

## One-time setup

### 1. Deploy the Worker
```bash
cd astrohour-worker
npm install
npm run deploy
```
Note the Worker URL: `https://astrohour-license.<your-subdomain>.workers.dev`

### 2. Set secrets
```bash
wrangler secret put STRIPE_WEBHOOK_SECRET
# paste: whsec_... from Stripe Dashboard

wrangler secret put STRIPE_SECRET_KEY
# paste: sk_live_...

wrangler secret put ED25519_PRIVATE_KEY_HEX
# paste: 2f60ac41825906d826b0e5e63e07db6eb794bd9a422e609968904b79414ec887
# (the 64-char hex private key from /home/exedev/astrohour-keys/)

wrangler secret put RESEND_API_KEY
# paste: re_...
```

### 3. Configure Stripe webhook
1. Stripe Dashboard > Developers > Webhooks > Add endpoint
2. URL: `https://astrohour-license.<your-subdomain>.workers.dev/stripe-webhook`
3. Events: `checkout.session.completed`
4. Copy the signing secret and set it via `wrangler secret put STRIPE_WEBHOOK_SECRET`

### 4. Verify
Use Stripe's "Send test webhook" with a `checkout.session.completed` event.
Check Wrangler logs: `wrangler tail`

## The Ed25519 keypair

The private key is at `/home/exedev/astrohour-keys/private.pem` (on the dev VM).
The public key (raw bytes) is embedded in `src-tauri/src/license.rs` as `PUBLIC_KEY`.

**Keep the private key secret.** Anyone with it can generate valid license keys.
Store it only in Wrangler secrets and in a secure password manager.

## Key format
`ASTROHOUR-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX`

14 groups of 8 base32 chars = 112 chars = 70 decoded bytes:
- bytes 0-5:   payload (version + reserved + epoch_days)
- bytes 6-69:  Ed25519 signature over payload

The app verifies offline using the embedded public key. No network call needed
after initial activation.
