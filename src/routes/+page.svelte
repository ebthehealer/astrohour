<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import {
    getDaySchedule, getCurrentHour, getMoonPhase,
    planetSymbol, phaseEmoji, phaseName, formatTime, minutesUntil,
    type DaySchedule, type PlanetaryHour, type MoonPhase,
  } from '$lib/api';

  let schedule: DaySchedule | null = null;
  let currentHour: PlanetaryHour | null = null;
  let moon: MoonPhase | null = null;
  let minsRemaining = 0;
  let unlisten: (() => void) | null = null;
  let tickInterval: ReturnType<typeof setInterval> | null = null;

  async function refresh() {
    [schedule, currentHour, moon] = await Promise.all([
      getDaySchedule(),
      getCurrentHour(),
      getMoonPhase(),
    ]);
    updateMins();
  }

  function updateMins() {
    if (currentHour) {
      minsRemaining = Math.round(minutesUntil(currentHour.end));
    }
  }

  onMount(async () => {
    await refresh();
    // Listen for 30-second backend tick
    unlisten = await listen('astro:tick', refresh);
    // Update the minutes-remaining counter every 30s locally too
    tickInterval = setInterval(updateMins, 30_000);
  });

  onDestroy(() => {
    unlisten?.();
    if (tickInterval) clearInterval(tickInterval);
  });

  $: dayRuler = schedule?.planetary_day ?? '';
  $: dayName = new Date().toLocaleDateString([], { weekday: 'long' });
  $: sunrise = schedule ? formatTime(schedule.sunrise) : '--:--';
  $: sunset  = schedule ? formatTime(schedule.sunset)  : '--:--';
</script>

<main>
  {#if !schedule || !currentHour || !moon}
    <div class="loading">Loading…</div>
  {:else}

    <!-- ── Current hour header ── -->
    <header class="current-hour">
      <span class="symbol">{planetSymbol(currentHour.planet)}</span>
      <div class="hour-info">
        <div class="hour-name">{currentHour.planet} Hour
          <span class="day-night">{currentHour.is_day ? 'Day' : 'Night'} {currentHour.sequence} of 12</span>
        </div>
        <div class="hour-times">
          {formatTime(currentHour.start)} → {formatTime(currentHour.end)}
          <span class="mins">{minsRemaining}m left</span>
        </div>
      </div>
    </header>

    <!-- ── Day ruler + Moon ── -->
    <section class="day-moon">
      <div class="day-ruler">
        <span class="label">{dayName}</span>
        <span class="ruler">{planetSymbol(dayRuler)} {dayRuler} Day</span>
        <span class="sun-times">\u2600\ufe0f {sunrise} → {sunset}</span>
      </div>
      <div class="moon-info">
        <span class="phase-emoji">{phaseEmoji(moon.phase)}</span>
        <div>
          <div class="phase-name">{phaseName(moon.phase)}</div>
          <div class="lunar-day">Moon Day {moon.lunar_day} &middot; {Math.round(moon.illumination * 100)}% lit</div>
        </div>
      </div>
    </section>

    <!-- ── All 24 hours ── -->
    <section class="all-hours">
      <div class="hours-label">ALL HOURS TODAY</div>
      <ul class="hours-list">
        {#each schedule.hours as h}
          {@const isCurrent = currentHour && h.index === currentHour.index}
          <li class="hour-row" class:current={isCurrent} class:night={!h.is_day} class:past={!isCurrent && new Date(h.end) < new Date()}>
            <span class="h-sym">{planetSymbol(h.planet)}</span>
            <span class="h-time">{formatTime(h.start)}</span>
            <span class="h-name">{h.planet} Hour</span>
            <span class="h-seq">{h.is_day ? 'Day' : 'Night'} {h.sequence}</span>
            {#if isCurrent}<span class="h-now">[now]</span>{/if}
          </li>
        {/each}
      </ul>
    </section>

  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    font-size: 13px;
    background: #1a1a2e;
    color: #e0e0e0;
    overflow-x: hidden;
  }

  main {
    width: 360px;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .loading {
    padding: 40px;
    text-align: center;
    color: #888;
  }

  /* Current hour */
  .current-hour {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
    background: #16213e;
    border-bottom: 1px solid #0f3460;
  }
  .symbol {
    font-size: 36px;
    line-height: 1;
    color: #e0b86a;
  }
  .hour-name {
    font-size: 16px;
    font-weight: 600;
    color: #fff;
  }
  .day-night {
    font-size: 11px;
    font-weight: 400;
    color: #888;
    margin-left: 6px;
  }
  .hour-times {
    font-size: 12px;
    color: #aaa;
    margin-top: 2px;
  }
  .mins {
    color: #e0b86a;
    margin-left: 8px;
    font-weight: 600;
  }

  /* Day + moon */
  .day-moon {
    padding: 12px 16px;
    background: #0f3460;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-bottom: 1px solid #1a4a80;
  }
  .day-ruler {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .label { color: #aaa; }
  .ruler { color: #e0b86a; font-weight: 600; }
  .sun-times { color: #aaa; font-size: 11px; margin-left: auto; }
  .moon-info {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .phase-emoji { font-size: 22px; }
  .phase-name { font-weight: 600; color: #fff; }
  .lunar-day { font-size: 11px; color: #aaa; }

  /* Hours list */
  .all-hours {
    flex: 1;
    overflow-y: auto;
    padding: 0 0 8px;
  }
  .hours-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: #555;
    padding: 10px 16px 4px;
    text-transform: uppercase;
  }
  .hours-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .hour-row {
    display: grid;
    grid-template-columns: 20px 48px 1fr auto auto;
    align-items: center;
    gap: 8px;
    padding: 5px 16px;
    border-bottom: 1px solid #1a1a2e;
    transition: background 0.15s;
  }
  .hour-row.current {
    background: #1a4a80;
    border-left: 3px solid #e0b86a;
    padding-left: 13px;
  }
  .hour-row.night { opacity: 0.8; }
  .hour-row.past  { opacity: 0.45; }
  .h-sym  { color: #e0b86a; font-size: 14px; text-align: center; }
  .h-time { color: #888; font-size: 11px; font-variant-numeric: tabular-nums; }
  .h-name { color: #ddd; }
  .h-seq  { color: #555; font-size: 10px; text-align: right; }
  .h-now  { color: #e0b86a; font-size: 10px; font-weight: 700; }
</style>
