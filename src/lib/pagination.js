/**
 * @typedef {{ chapterIndex: number, pageIndex: number }} ReadingPosition
 */

/** @param {number} value */
export function clampNonNegativeInteger(value) {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.floor(value));
}

/**
 * @param {number} value
 * @param {number} count
 */
export function clampIndex(value, count) {
  const max = Math.max(0, clampNonNegativeInteger(count) - 1);
  return Math.min(max, clampNonNegativeInteger(value));
}

/**
 * @param {number} pageIndex
 * @param {number} pageCount
 */
export function clampPage(pageIndex, pageCount) {
  return clampIndex(pageIndex, pageCount);
}

/**
 * @param {number} containerHeight
 * @param {number} fontSize
 */
export function calculatePageStep(containerHeight, fontSize) {
  return Math.max(1, containerHeight - Math.round(fontSize * 1.75));
}

/**
 * @param {number} scrollHeight
 * @param {number} pageStep
 * @param {number} [containerHeight]
 */
export function calculatePageCount(scrollHeight, pageStep, containerHeight = pageStep) {
  const maxOffset = Math.max(0, Math.max(0, scrollHeight) - Math.max(1, containerHeight));
  return Math.max(1, Math.ceil(maxOffset / Math.max(1, pageStep)) + 1);
}

/**
 * @param {number} scrollHeight
 * @param {number} containerHeight
 * @param {number} pageStep
 * @param {number} pageIndex
 */
export function calculatePageOffset(scrollHeight, containerHeight, pageStep, pageIndex) {
  const requestedOffset = clampNonNegativeInteger(pageIndex) * Math.max(1, pageStep);
  const maxOffset = Math.max(0, Math.max(0, scrollHeight) - Math.max(1, containerHeight));
  return Math.min(requestedOffset, maxOffset);
}

/**
 * @param {number} chapterCount
 * @param {number} chapterIndex
 * @param {number} pageIndex
 * @param {number} pageCount
 */
export function calculateOverallProgress(chapterCount, chapterIndex, pageIndex, pageCount) {
  if (chapterCount <= 0) return 0;
  const safeChapter = clampIndex(chapterIndex, chapterCount);
  const safePage = clampPage(pageIndex, pageCount);
  return Math.min(
    100,
    Math.round(((safeChapter + (safePage + 1) / Math.max(1, pageCount)) / chapterCount) * 100)
  );
}

/**
 * @param {ReadingPosition} position
 * @param {number} chapterCount
 * @param {number} pageCount
 * @returns {ReadingPosition}
 */
export function clampReadingPosition(position, chapterCount, pageCount) {
  return {
    chapterIndex: clampIndex(position.chapterIndex, chapterCount),
    pageIndex: clampPage(position.pageIndex, pageCount),
  };
}

/**
 * @param {ReadingPosition} position
 * @param {number} chapterCount
 * @param {number} pageCount
 * @returns {ReadingPosition}
 */
export function nextReadingPosition(position, chapterCount, pageCount) {
  const current = clampReadingPosition(position, chapterCount, pageCount);
  if (current.pageIndex < Math.max(1, pageCount) - 1) {
    return { ...current, pageIndex: current.pageIndex + 1 };
  }
  if (current.chapterIndex < Math.max(1, chapterCount) - 1) {
    return { chapterIndex: current.chapterIndex + 1, pageIndex: 0 };
  }
  return current;
}

/**
 * @param {ReadingPosition} position
 * @param {number} chapterCount
 * @param {number} pageCount
 * @returns {ReadingPosition}
 */
export function previousReadingPosition(position, chapterCount, pageCount) {
  const current = clampReadingPosition(position, chapterCount, pageCount);
  if (current.pageIndex > 0) {
    return { ...current, pageIndex: current.pageIndex - 1 };
  }
  if (current.chapterIndex > 0) {
    return { chapterIndex: current.chapterIndex - 1, pageIndex: 0 };
  }
  return current;
}

/**
 * @param {number} percent
 * @param {number} chapterCount
 * @param {number} currentChapterIndex
 * @param {number} currentPageCount
 * @returns {ReadingPosition}
 */
export function positionFromPercent(percent, chapterCount, currentChapterIndex, currentPageCount) {
  if (chapterCount <= 0) return { chapterIndex: 0, pageIndex: 0 };
  const pct = Math.max(0, Math.min(100, percent));
  const fraction = (pct / 100) * chapterCount;
  const targetChapter = clampIndex(Math.floor(fraction), chapterCount);
  const remainder = fraction - targetChapter;
  const pageIndex = targetChapter === currentChapterIndex
    ? Math.round(remainder * Math.max(1, currentPageCount)) - 1
    : 0;
  return clampReadingPosition(
    { chapterIndex: targetChapter, pageIndex },
    chapterCount,
    targetChapter === currentChapterIndex ? currentPageCount : 1
  );
}
