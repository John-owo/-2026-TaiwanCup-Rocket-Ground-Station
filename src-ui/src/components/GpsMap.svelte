<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import * as L from 'leaflet';
  import { store } from '@/lib/stores.svelte';
  import { appendTrackPoint, haversineDistanceMeters, isValidGpsPosition } from '@/lib/gps-map.js';
  import { bearingDegrees, compassLabel, formatDistance } from '@/lib/flight-display.js';

  type GpsPosition = { lat: number; lng: number };

  let mapContainer: HTMLDivElement;
  let map: L.Map | undefined;
  let tileLayer: L.TileLayer | undefined;
  let marker: L.Marker | undefined;
  let launchMarker: L.Marker | undefined;
  let trackLine: L.Polyline | undefined;
  let resizeObserver: ResizeObserver | undefined;
  let trackPoints: GpsPosition[] = [];
  let lastValidPosition = $state<GpsPosition | null>(null);
  let launchPoint = $state<GpsPosition | null>(null);
  let observedSessionId: number | null = null;

  let following = $state(true);
  let fixValid = $state(false);
  let mapError = $state('');
  let lastFixTime = $state('--');
  let trackCount = $state(0);

  let distance = $derived(
    launchPoint && lastValidPosition ? haversineDistanceMeters(launchPoint, lastValidPosition) : null,
  );
  let bearing = $derived(
    launchPoint && lastValidPosition && distance !== null && distance >= 1
      ? bearingDegrees(launchPoint, lastValidPosition)
      : null,
  );

  const rocketIcon = L.divIcon({
    className: 'rocket-marker-shell',
    html: '<div class="rocket-marker"></div>',
    iconSize: [22, 22],
    iconAnchor: [11, 11],
  });

  const launchIcon = L.divIcon({
    className: 'rocket-marker-shell',
    html: '<div class="launch-marker"></div>',
    iconSize: [18, 18],
    iconAnchor: [9, 9],
  });

  onMount(() => {
    map = L.map(mapContainer, { zoomControl: false, attributionControl: true }).setView([23.7, 121.0], 7);
    L.control.zoom({ position: 'topright' }).addTo(map);
    tileLayer = L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
      maxZoom: 19,
      attribution: '&copy; OpenStreetMap contributors',
      className: 'chart-tiles',
    }).addTo(map);
    trackLine = L.polyline([], { color: getComputedStyle(document.documentElement).getPropertyValue('--rocket').trim() || '#a3166f', weight: 3 }).addTo(map);

    tileLayer.on('tileerror', handleTileError);
    tileLayer.on('load', handleTileLoad);
    map.on('dragstart', disableFollowing);

    resizeObserver = new ResizeObserver(() => map?.invalidateSize());
    resizeObserver.observe(mapContainer);

    const themeObserver = new MutationObserver(() => {
      const color = getComputedStyle(document.documentElement).getPropertyValue('--rocket').trim();
      if (color) trackLine?.setStyle({ color });
    });
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });

    if (lastValidPosition) renderPosition(lastValidPosition, true);

    return () => {
      themeObserver.disconnect();
      resizeObserver?.disconnect();
      tileLayer?.off('tileerror', handleTileError);
      tileLayer?.off('load', handleTileLoad);
      map?.off('dragstart', disableFollowing);
      map?.remove();
      map = undefined;
      tileLayer = undefined;
      marker = undefined;
      launchMarker = undefined;
      trackLine = undefined;
    };
  });

  $effect(() => {
    const revision = store.telemetryRevision;
    if (revision === 0) return;
    const telemetry = untrack(() => store.telemetry);
    if (observedSessionId !== telemetry.sessionId) {
      // A new airborne session starts a new launch reference.
      observedSessionId = telemetry.sessionId;
      launchPoint = null;
      launchMarker?.remove();
      launchMarker = undefined;
    }
    const position = { lat: telemetry.latitude, lng: telemetry.longitude };
    if (!isValidGpsPosition(position)) {
      fixValid = false;
      return;
    }

    const firstFix = untrack(() => lastValidPosition === null);
    lastValidPosition = position;
    fixValid = true;
    lastFixTime = new Date().toLocaleTimeString('zh-TW', { hour12: false });
    if (untrack(() => launchPoint === null)) {
      launchPoint = position;
      if (map) launchMarker = L.marker(position, { icon: launchIcon, interactive: false }).addTo(map);
    }

    const nextTrack = appendTrackPoint(trackPoints, position);
    if (nextTrack !== trackPoints) {
      trackPoints = nextTrack;
      trackCount = trackPoints.length;
      trackLine?.setLatLngs(trackPoints);
    }
    untrack(() => renderPosition(position, firstFix));
  });

  function renderPosition(position: GpsPosition, firstFix: boolean) {
    if (!map) return;
    if (!marker) marker = L.marker(position, { icon: rocketIcon }).addTo(map);
    else marker.setLatLng(position);

    if (following) {
      map.setView(position, firstFix ? 17 : map.getZoom(), { animate: false });
    }
  }

  function disableFollowing() {
    following = false;
  }

  function locateRocket() {
    if (!lastValidPosition || !map) return;
    following = true;
    map.setView(lastValidPosition, Math.max(map.getZoom(), 16), { animate: false });
  }

  function toggleFollowing() {
    following = !following;
    if (following) locateRocket();
  }

  function clearTrack() {
    trackPoints = [];
    trackCount = 0;
    trackLine?.setLatLngs([]);
  }

  function handleTileError() {
    mapError = '地圖圖磚載入失敗（可能沒有網路），位置數值仍持續更新';
  }

  function handleTileLoad() {
    mapError = '';
  }
</script>

<section class="gps" aria-label="GPS 位置">
  <div class="map" bind:this={mapContainer}></div>

  <div class="tag">
    <h2>GPS 位置</h2>
    <p>
      <span class:valid={fixValid}>{fixValid ? '定位有效' : '等待有效定位'}</span>
      · 更新 <span class="num">{lastFixTime}</span> · 軌跡 <span class="num">{trackCount}</span> 點
    </p>
  </div>

  {#if mapError}
    <p class="map-error" role="status">{mapError}</p>
  {/if}

  <div class="dock">
    <div class="tools">
      <button class:active={following} onclick={toggleFollowing} aria-pressed={following}>自動跟隨</button>
      <button onclick={locateRocket} disabled={!lastValidPosition}>定位火箭</button>
      <button onclick={clearTrack} disabled={trackCount === 0}>清除軌跡</button>
    </div>
    <dl class="recover">
      <div title="以本空中 session 第一筆有效定位作為發射點"><dt>距發射點</dt><dd class="num">{distance === null ? '--' : formatDistance(distance)}</dd></div>
      <div><dt>方位</dt><dd class="num">{bearing === null ? '--' : `${Math.round(bearing)}° ${compassLabel(bearing)}`}</dd></div>
      <div><dt>緯度 / 經度</dt><dd class="num coords">{#if lastValidPosition}{lastValidPosition.lat.toFixed(5)}<br />{lastValidPosition.lng.toFixed(5)}{:else}--{/if}</dd></div>
    </dl>
  </div>
</section>

<style>
  .gps {
    position: relative;
    min-height: 0;
    overflow: hidden;
    border-bottom: 1px solid var(--rule);
    background: var(--paper-2);
  }

  .map { position: absolute; inset: 0; background: var(--paper); }
  .map :global(.chart-tiles) { filter: var(--map-filter); mix-blend-mode: multiply; }
  :global(:root[data-theme="dark"]) .map :global(.chart-tiles) { mix-blend-mode: screen; }

  .tag {
    position: absolute;
    top: 12px;
    left: 12px;
    z-index: 500;
    max-width: calc(100% - 70px);
    padding: 8px 11px;
    border: 1px solid var(--rule);
    border-radius: var(--radius);
    background: var(--sheet);
    box-shadow: var(--shadow-pop);
  }
  .tag h2 { font-size: 15px; font-weight: 700; line-height: 1.2; }
  .tag p { margin-top: 3px; color: var(--ink-2); font-size: 12px; }
  .tag p span:first-child { color: var(--warn); font-weight: 600; }
  .tag p span.valid { color: var(--live); }

  .map-error {
    position: absolute;
    top: 74px;
    left: 12px;
    right: 12px;
    z-index: 500;
    padding: 8px 10px;
    border-radius: var(--radius);
    background: var(--sheet);
    color: var(--warn);
    font-size: 12.5px;
    box-shadow: var(--shadow-pop);
  }

  .dock {
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 22px;
    z-index: 500;
    display: grid;
    gap: 8px;
  }

  .tools { display: flex; gap: 6px; }
  .tools button {
    height: 30px;
    padding: 0 10px;
    border: 1px solid var(--rule);
    border-radius: var(--radius);
    background: var(--sheet);
    color: var(--ink-2);
    font-size: 12.5px;
    box-shadow: var(--shadow-pop);
  }
  .tools button:hover:not(:disabled) { color: var(--ink); border-color: var(--rule-strong); }
  .tools button.active { border-color: var(--ink); background: var(--ink); color: var(--paper); }
  .tools button:disabled { opacity: .55; }

  .recover {
    display: grid;
    grid-template-columns: 1fr 1.15fr 1fr;
    border: 1px solid var(--rule);
    border-radius: var(--radius);
    background: var(--sheet);
    box-shadow: var(--shadow-pop);
  }
  .recover div { min-width: 0; padding: 8px 11px; border-right: 1px solid var(--rule); }
  .recover div:last-child { border-right: 0; }
  dt { color: var(--ink-3); font-size: 11.5px; }
  dd { overflow: hidden; font-size: 17px; font-weight: 600; line-height: 1.25; text-overflow: ellipsis; white-space: nowrap; }
  dd.coords { font-size: 13px; font-weight: 500; line-height: 1.25; }

  .gps :global(.leaflet-control-zoom) { border: 1px solid var(--rule) !important; border-radius: var(--radius); box-shadow: var(--shadow-pop); }
  .gps :global(.leaflet-control-zoom a) { background: var(--sheet); color: var(--ink); border-color: var(--rule); }
  .gps :global(.leaflet-control-attribution) { background: color-mix(in srgb, var(--sheet) 85%, transparent); color: var(--ink-3); font-size: 10px; }
  .gps :global(.leaflet-control-attribution a) { color: var(--ink-2); }

  :global(.rocket-marker-shell) { background: transparent; border: 0; }
  :global(.rocket-marker) {
    width: 22px;
    height: 22px;
    border: 3px solid var(--sheet);
    border-radius: 50%;
    background: var(--rocket);
    box-shadow: 0 2px 8px rgba(15, 27, 45, .35);
  }
  :global(.launch-marker) {
    width: 18px;
    height: 18px;
    border: 2.5px solid var(--ink);
    border-radius: 50%;
    background: radial-gradient(circle, var(--ink) 0 2.5px, var(--sheet) 3px);
  }
</style>
