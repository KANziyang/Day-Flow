import { version } from "../../src-tauri/tauri.conf.json";

// 与安装器使用同一构建版本，避免界面和发布包分别维护版本文字。
export const APP_VERSION_LABEL = `V${version}`;
