// Spin a wheel of choices around the pointer and pick one by angle.
type Slice = { label: string; run: () => Promise<void> };

export function pick(slices: Slice[], x: number, y: number, cx: number, cy: number): Slice | null {
	const distance = Math.hypot(x - cx, y - cy);
	if (distance < 62) return null; // the middle cancels

	const step = (Math.PI * 2) / slices.length;
	const angle = (Math.atan2(y - cy, x - cx) + Math.PI / 2 + step / 2 + Math.PI * 2) % (Math.PI * 2);
	return slices[Math.floor(angle / step)];
}

const convert: Slice[] = ["JPG", "PNG", "WebP"].map((label) => ({
	label,
	run: async () => console.log(`Converting to ${label}`),
}));

await pick(convert, 320, 140, 260, 258)?.run();
