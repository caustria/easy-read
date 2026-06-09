/**
 * @typedef {{ start: number, end: number }} TextRangeOffsets
 * @typedef {{ node: Text, start: number, end: number }} TextSegment
 */

/** @param {Node} root */
export function textLength(root) {
  let length = 0;
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  while (walker.nextNode()) {
    length += walker.currentNode.textContent?.length ?? 0;
  }
  return length;
}

/**
 * @param {Node} root
 * @param {Node} targetNode
 * @param {number} targetOffset
 */
export function getTextOffset(root, targetNode, targetOffset) {
  try {
    const range = document.createRange();
    range.selectNodeContents(root);
    range.setEnd(targetNode, targetOffset);
    return range.toString().length;
  } catch (_) {
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    let count = 0;
    while (walker.nextNode()) {
      if (walker.currentNode === targetNode) return count + targetOffset;
      count += walker.currentNode.textContent?.length ?? 0;
    }
    return count;
  }
}

/**
 * @param {Node} root
 * @param {number} start
 * @param {number} end
 */
export function isValidTextRange(root, start, end) {
  if (!Number.isFinite(start) || !Number.isFinite(end)) return false;
  if (start < 0 || end <= start) return false;
  return end <= textLength(root);
}

/**
 * @param {Node} root
 * @param {number} start
 * @param {number} end
 * @returns {TextSegment[]}
 */
export function textSegmentsForRange(root, start, end) {
  if (!isValidTextRange(root, start, end)) return [];

  /** @type {TextSegment[]} */
  const segments = [];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let count = 0;

  while (walker.nextNode()) {
    const node = /** @type {Text} */ (walker.currentNode);
    const length = node.textContent?.length ?? 0;
    const nodeStart = count;
    const nodeEnd = count + length;

    if (nodeEnd > start && nodeStart < end) {
      segments.push({
        node,
        start: Math.max(0, start - nodeStart),
        end: Math.min(length, end - nodeStart),
      });
    }

    count = nodeEnd;
    if (count >= end) break;
  }

  return segments;
}

/**
 * Wraps a text offset range by splitting and wrapping each affected text node.
 * This avoids Range.surroundContents(), which fails when the range crosses
 * element boundaries.
 *
 * @param {Node} root
 * @param {number} start
 * @param {number} end
 * @param {() => HTMLElement} createWrapper
 * @returns {HTMLElement[]}
 */
export function wrapTextRange(root, start, end, createWrapper) {
  const segments = textSegmentsForRange(root, start, end);
  /** @type {HTMLElement[]} */
  const wrappers = [];

  for (const segment of segments.reverse()) {
    const text = segment.node;
    const parent = text.parentNode;
    if (!parent || segment.start >= segment.end) continue;

    let target = text;
    const length = target.textContent?.length ?? 0;
    if (segment.end < length) target.splitText(segment.end);
    if (segment.start > 0) target = target.splitText(segment.start);

    const wrapper = createWrapper();
    parent.insertBefore(wrapper, target);
    wrapper.appendChild(target);
    wrappers.unshift(wrapper);
  }

  return wrappers;
}

/** @param {HTMLElement} mark */
export function unwrapMark(mark) {
  const parent = mark.parentNode;
  if (!parent) return;
  while (mark.firstChild) parent.insertBefore(mark.firstChild, mark);
  parent.removeChild(mark);
  parent.normalize();
}

/**
 * @param {ParentNode} root
 * @param {string} selector
 */
export function unwrapMarks(root, selector) {
  for (const mark of Array.from(root.querySelectorAll(selector)).reverse()) {
    if (mark instanceof HTMLElement) unwrapMark(mark);
  }
}

/**
 * @param {Node} root
 * @param {string} query
 * @param {{ caseSensitive?: boolean }} [options]
 * @returns {TextRangeOffsets | null}
 */
export function findTextRange(root, query, options = {}) {
  if (!query) return null;
  const text = root.textContent ?? "";
  const haystack = options.caseSensitive ? text : text.toLowerCase();
  const needle = options.caseSensitive ? query : query.toLowerCase();
  const index = haystack.indexOf(needle);
  if (index < 0) return null;
  return { start: index, end: index + query.length };
}
