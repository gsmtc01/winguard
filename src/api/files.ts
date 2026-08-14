import { invoke } from "@tauri-apps/api/core";

/** UTF-8 텍스트를 지정 경로에 파일로 저장한다. */
export const writeTextFile = (path: string, content: string): Promise<void> =>
  invoke<void>("write_text_file", { path, content });
