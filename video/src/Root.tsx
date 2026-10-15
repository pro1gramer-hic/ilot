import { Composition } from "remotion";
import { Demo, DEMO_FRAMES } from "./Demo";

export const Root = () => (
  <Composition id="IlotDemo" component={Demo} durationInFrames={DEMO_FRAMES} fps={30} width={1920} height={1080} />
);
