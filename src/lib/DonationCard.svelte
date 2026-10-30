<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { DONATE_URL, dismissDonation, recordDonationClick } from './api';

  const dispatch = createEventDispatcher<{ close: void }>();

  /**
   * The quiet period is recorded before the browser is opened, so it starts
   * whether or not the link actually loads. A donation the app never saw would
   * otherwise mean another card next week.
   */
  async function donate() {
    await recordDonationClick();
    window.open(DONATE_URL, '_blank');
    dispatch('close');
  }

  // "Not now" needs no call: the backend already recorded this occurrence as
  // shown, so the next card is simply the next date.
  function notNow() {
    dispatch('close');
  }

  async function never() {
    await dismissDonation(true);
    dispatch('close');
  }
</script>

<div class="scrim" role="presentation" on:click={notNow}>
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="card" role="dialog" tabindex="-1" aria-modal="true" aria-labelledby="donation-title"
    on:click|stopPropagation>

    <div class="glyph" aria-hidden="true">♄</div>

    <h2 id="donation-title">AstroHour is free</h2>
    <p class="lede">
      Every feature, on every platform, with no licence and no account.
    </p>
    <p class="ask">
      If it earns a place in your day, a donation keeps it going.
    </p>

    <button class="donate" on:click={donate}>♥ Donate</button>

    <div class="dismiss">
      <button on:click={notNow}>Not now</button>
      <button on:click={never}>Don't ask again</button>
    </div>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
    background: rgba(0, 0, 0, 0.62);
  }

  .card {
    width: 100%;
    max-width: 330px;
    padding: 26px 24px 18px;
    border: 1px solid var(--bg-highlight, #1a4a80);
    border-radius: 12px;
    background: var(--bg-surface, #16213e);
    color: var(--text-primary, #ddd);
    text-align: center;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.55);
  }

  .glyph {
    font-size: 30px;
    line-height: 1;
    color: var(--accent, #e0b86a);
  }

  h2 {
    margin: 12px 0 10px;
    font-size: 17px;
    font-weight: 600;
    letter-spacing: 0.01em;
  }

  p {
    margin: 0 0 10px;
    font-size: 12.5px;
    line-height: 1.55;
    color: var(--text-secondary, #aab);
  }

  .ask {
    color: var(--text-muted, #889);
  }

  .donate {
    width: 100%;
    margin-top: 6px;
    padding: 10px;
    border: none;
    border-radius: 7px;
    background: var(--accent, #e0b86a);
    color: #1a1205;
    font-family: inherit;
    font-size: 13.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .donate:hover { filter: brightness(1.08); }

  .dismiss {
    display: flex;
    justify-content: center;
    gap: 18px;
    margin-top: 14px;
  }

  .dismiss button {
    padding: 4px 2px;
    border: none;
    background: none;
    color: var(--text-muted, #889);
    font-family: inherit;
    font-size: 11.5px;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .dismiss button:hover { color: var(--text-primary, #ddd); }
</style>
