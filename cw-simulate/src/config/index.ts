import { config as dotenvLib } from "dotenv";
import * as fs from "fs";

const pathEnv = [`.env.${process.env.NODE_ENV}`, ".env"];
const baseDir = __dirname + "/../../";
const getPathEnv = () => {
  for (const path of pathEnv) {
    if (fs.existsSync(path)) {
      return path;
    }
    if (fs.existsSync(baseDir + path)) {
      return baseDir + path;
    }
  }
  return null;
};
const configDotenv: any = {
  path: getPathEnv(),
};
dotenvLib(configDotenv);
if (configDotenv.path) {
  console.log("++++ Load file env", configDotenv.path);
}

const configDefault = {
  basedir: baseDir,
  appdir: baseDir + "src/",
  isProd:
    process.env.APP_ENV &&
    (process.env.APP_ENV.toLowerCase() === "prod" ||
      process.env.APP_ENV.toLowerCase() === "production")
      ? true
      : false,
};

const config: any = Object.assign(configDefault, process.env);

export default config;
