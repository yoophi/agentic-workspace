export function stripInlineMarkdown(text: string): string {
  return (
    text
      // 이미지가 링크 문법을 포함하므로 링크보다 먼저 처리한다.
      .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
      .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
      .replace(/`([^`]+)`/g, "$1")
      .replace(/(\*\*|__)(.+?)\1/g, "$2")
      .replace(/~~(.+?)~~/g, "$1")
      .replace(/\*([^*]+)\*/g, "$1")
      // snake_case 식별자를 보존하기 위해 단어 경계의 underscore 강조만 제거한다.
      .replace(/(^|\s)_([^_]+)_(?=\s|$)/g, "$1$2")
      .trim()
  );
}
