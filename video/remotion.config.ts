// The demo draws Mochi with îlot's own engine (../src/lib/mochi). Its moves read their
// texts through a Svelte store: outside Svelte, a plain stand-in gives the French ones.
import path from "node:path";
import { Config } from "@remotion/cli/config";
import webpack from "webpack";

Config.setVideoImageFormat("jpeg");
Config.setJpegQuality(92);
Config.overrideWebpackConfig((config) => ({
  ...config,
  plugins: [
    ...(config.plugins ?? []),
    new webpack.NormalModuleReplacementPlugin(/i18n[\\/]live\.svelte$/, path.resolve(process.cwd(), "src/live-shim.ts")),
  ],
}));
