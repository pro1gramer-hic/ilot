// The AIs îlot can plug into (`list_connectors`), shared by the Agents tab and the header.
// Nothing is ever plugged without the user's click on a switch or a button; before any
// change, the core saves the AI's config (`.avant-ilot`).

import { type AiSource, type Plug, plugState, targetsToFlip } from "../../lib/agents";
import { type ConnectorView, listConnectors, setConnected } from "../../lib/ipc";

class Connectors {
  list = $state<ConnectorView[] | null>(null);
  error = $state<string | null>(null);
  /** The AI being plugged or unplugged. */
  busy = $state<string | null>(null);

  view(source: AiSource): ConnectorView | undefined {
    return this.list?.find((c) => c.id === source);
  }

  plug(source: AiSource): Plug {
    return plugState(this.view(source));
  }

  /** How many AIs are plugged somewhere. */
  connected = $derived(this.list?.filter((c) => plugState(c) === "on").length ?? 0);

  async load(): Promise<void> {
    try {
      this.list = await listConnectors();
      this.error = null;
    } catch (error) {
      this.error = String(error);
    }
  }

  /** Plugs an AI everywhere it is installed, or unplugs it everywhere. */
  async switch(source: AiSource, on: boolean): Promise<void> {
    const view = this.view(source);
    if (!view || this.busy) return;
    this.busy = source;
    try {
      for (const target of targetsToFlip(view, on)) this.list = await setConnected(source, target, on);
      this.error = null;
    } catch (error) {
      this.error = String(error);
    } finally {
      this.busy = null;
    }
  }
}

export const connectors = new Connectors();
