import { invoke } from "@tauri-apps/api/core";

/** 창 제목 적용(데스크톱 표현, 044 T029): 창 제목과 네이티브 Window 메뉴를 함께 맞춘다. 서버는 창에 제목을 넣지 않는다. */
export function applyWindowTitle(title: string) {
  return invoke<void>("apply_window_title", { title });
}
