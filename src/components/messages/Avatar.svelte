<!-- A sender in the Messages tab: the first letter of the name in a round tile, and a small
     dot in the app's color at its corner (appColor). Decorative: the name and the app are
     always written next to it. -->
<script lang="ts">
  import { appColor } from "../../lib/notifications";

  let { sender, app, size = 32 }: { sender: string; app: string; size?: number } = $props();

  const initial = $derived([...sender.trim()][0]?.toLocaleUpperCase("fr") ?? "?");
</script>

<span class="avatar" style:--size="{size}px" style:--app={appColor(app)} aria-hidden="true">{initial}</span>

<style>
  .avatar {
    position: relative;
    flex: none;
    width: var(--size);
    height: var(--size);
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--surface-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      var(--hi);
    color: var(--ink);
    font-size: var(--fs-3);
    font-weight: 600;
  }

  /* The app's dot, cut out of the tile by a ring of the background. */
  .avatar::after {
    content: "";
    position: absolute;
    right: -2px;
    bottom: -2px;
    width: var(--s2);
    height: var(--s2);
    border-radius: 50%;
    background: var(--app);
    box-shadow: 0 0 0 2px var(--avatar-ring, var(--panel));
  }
</style>
