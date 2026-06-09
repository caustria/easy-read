import { cubicOut } from "svelte/easing";

export const leftPanelTransition = { x: -300, duration: 150, easing: cubicOut, opacity: 1 };
export const rightPanelTransition = { x: 300, duration: 150, easing: cubicOut, opacity: 1 };
export const modalTransition = { y: -20, duration: 150 };
