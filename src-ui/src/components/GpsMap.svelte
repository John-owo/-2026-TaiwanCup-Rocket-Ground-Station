<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import * as L from 'leaflet';
  import { store } from '@/lib/stores.svelte';
  import { appendTrackPoint, isValidGpsPosition } from '@/lib/gps-map.js';

  type GpsPosition = { lat: number; lng: number };

  let mapContainer: HTMLDivElement;
  let map: L.Map | undefined;
  let tileLayer: L.TileLayer | undefined;
  let marker: L.Marker | undefined;
  let trackLine: L.Polyline | undefined;
  let resizeObserver: ResizeObserver | undefined;
  let trackPoints: GpsPosition[] = [];
  let lastValidPosition = $state<GpsPosition | null>(null);

  let following = $state(true);
  let positionStatus = $state('等待有效定位');
  let mapError = $state('');
  let lastFixTime = $state('--');
  let trackCount = $state(0);

  const rocketIcon = L.divIcon({
    className: 'rocket-marker-shell',
    html: '<div class="rocket-marker" aria-label="火箭位置">▲</div>',
    iconSize: [28, 28],
    iconAnchor: [14, 14],
  });

  onMount(() => {
    map = L.map(mapContainer, { zoomControl: true }).setView([23.7, 121.0], 7);
    tileLayer = L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
      maxZoom: 19,
      attribution: '&copy; OpenStreetMap contributors',
    }).addTo(map);
    trackLine = L.polyline([], { color: '#73d2b6', weight: 3 }).addTo(map);

    tileLayer.on('tileerror', handleTileError);
    tileLayer.on('load', handleTileLoad);
    map.on('dragstart', disableFollowing);

    resizeObserver = new ResizeObserver(() => map?.invalidateSize());
    resizeObserver.observe(mapContainer);

    if (lastValidPosition) renderPosition(lastValidPosition, true);

    return () => {
      resizeObserver?.disconnect();
      tileLayer?.off('tileerror', handleTileError);
      tileLayer?.off('load', handleTileLoad);
      map?.off('dragstart', disableFollowing);
      map?.remove();
      map = undefined;
      tileLayer = undefined;
      marker = undefined;
      trackLine = undefined;
    };
  });

  $effect(() => {
    const revision = store.telemetryRevision;
    if (revision === 0) return;
    const telemetry = untrack(() => store.telemetry);
    const position = { lat: telemetry.latitude, lng: telemetry.longitude };
    if (!isValidGpsPosition(position)) {
      positionStatus = '等待有效定位';
      return;
    }

    const firstFix = untrack(() => lastValidPosition === null);
    lastValidPosition = position;
    positionStatus = '定位有效';
    lastFixTime = new Date().toLocaleTimeString('zh-TW', { hour12: false });

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
      map.setView(position, firstFix ? 16 : map.getZoom(), { animate: false });
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
    mapError = '地圖載入失敗，GPS 數值仍持續更新';
  }

  function handleTileLoad() {
    mapError = '';
  }
</script>

<article class="gps-card">
  <div class="card-header">
    <div>
      <span class="eyebrow">POSITION TRACKING</span>
      <span class="header-label">GPS 即時位置</span>
      <span class="status" class:valid={positionStatus === '定位有效'}>{positionStatus}</span>
    </div>
    <span class="track-count mono">軌跡 {trackCount}</span>
  </div>

  <div class="map-wrap">
    <div class="map" bind:this={mapContainer}></div>
    {#if mapError}
      <div class="map-error">{mapError}</div>
    {/if}
  </div>

  <div class="gps-readout">
    <span class="mono">經度 {lastValidPosition ? lastValidPosition.lng.toFixed(6) : '--'}</span>
    <span class="mono">緯度 {lastValidPosition ? lastValidPosition.lat.toFixed(6) : '--'}</span>
    <span class="mono">地速 {store.telemetry.groundSpeed.toFixed(1)} m/s</span>
    <span class="mono">更新 {lastFixTime}</span>
  </div>

  <div class="map-actions">
    <button class:active={following} onclick={toggleFollowing}>
      {following ? '自動跟隨：開' : '自動跟隨：關'}
    </button>
    <button onclick={locateRocket} disabled={!lastValidPosition}>定位火箭</button>
    <button onclick={clearTrack} disabled={trackCount === 0}>清除軌跡</button>
  </div>
</article>

<style>
  .gps-card {
    overflow: hidden;
    border: 1px solid var(--glass-border);
    border-radius: var(--radius-lg);
    background: var(--glass-bg);
    box-shadow: var(--glass-shadow);
  }

  .card-header,
  .card-header > div,
  .map-actions {
    display: flex;
    align-items: center;
  }
  .card-header { justify-content: space-between; gap: var(--sp-2); padding: var(--sp-4) var(--sp-5); }
  .card-header > div { flex-wrap: wrap; gap: var(--sp-2); }
  .eyebrow { width: 100%; color: var(--accent-cyan); font-family: var(--font-mono); font-size: 9px; letter-spacing: .13em; }
  .header-label { color: var(--text-primary); font-size: var(--fs-md); font-weight: 560; }
  .status { color: var(--accent-orange); font-size: var(--fs-xs); }
  .status.valid { color: var(--accent-green); }
  .track-count { color: var(--text-tertiary); font-size: var(--fs-xs); }

  .map-wrap { position: relative; overflow: hidden; border-block: 1px solid var(--border-muted); }
  .map { width: 100%; height: clamp(240px, 30vh, 330px); background: var(--surface); filter: saturate(.72) brightness(.82) contrast(1.08); }
  .map-error {
    position: absolute;
    right: var(--sp-2);
    bottom: var(--sp-2);
    left: var(--sp-2);
    z-index: 500;
    padding: var(--sp-2);
    border-radius: var(--radius-sm);
    background: rgba(10, 14, 26, 0.9);
    color: var(--accent-orange);
    font-size: var(--fs-xs);
    text-align: center;
  }

  .gps-readout { display: grid; grid-template-columns: 1fr 1fr; gap: var(--sp-1); padding: var(--sp-3) var(--sp-5) 0; color: var(--text-secondary); font-size: 10px; }
  .map-actions { flex-wrap: wrap; gap: var(--sp-2); padding: var(--sp-3) var(--sp-5) var(--sp-4); }
  .map-actions button {
    flex: 1;
    min-width: 78px;
    padding: var(--sp-2);
    border: 1px solid var(--surface-border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-secondary);
    font-size: 10px;
  }
  .map-actions button.active { border-color: var(--accent-cyan); color: var(--accent-cyan); }
  .map-actions button:disabled { cursor: not-allowed; opacity: 0.45; }

  :global(.rocket-marker-shell) { background: transparent; border: 0; }
  :global(.rocket-marker) {
    display: grid;
    width: 28px;
    height: 28px;
    place-items: center;
    border: 2px solid #00141a;
    border-radius: 50%;
    background: var(--accent-cyan);
    box-shadow: 0 4px 12px rgba(0, 0, 0, .35);
    color: #00141a;
    font-size: 16px;
    transform: rotate(0deg);
  }

  @media (max-height: 900px) and (min-width: 1241px) {
    .card-header { padding: var(--sp-3) var(--sp-4); }
    .map { height: clamp(150px, 18vh, 190px); }
    .gps-readout { padding: var(--sp-2) var(--sp-4) 0; }
    .map-actions { padding: var(--sp-2) var(--sp-4) var(--sp-3); }
  }
</style>
