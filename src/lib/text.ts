const RICH_TEXT_TAG =
  /<\/?(?:align|alpha|b|br|color|font|i|indent|line-height|line-indent|link|lowercase|margin|mark|mspace|nobr|noparse|pos|quad|rotate|s|size|smallcaps|space|sprite|style|sub|sup|u|uppercase|voffset|width)(?:\s+[^>]*)?(?:=[^>]*)?>/gi;

export function cleanDisplayText(value: string): string {
  return value.replace(RICH_TEXT_TAG, "").split(/\s+/).filter(Boolean).join(" ");
}
