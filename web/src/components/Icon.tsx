/** Path data for a 24 by 24 grid, drawn with a 1.75 stroke. */
const PATHS = {
  sliders: "M4 7h9M17 7h3M4 17h3M11 17h9M15 4.5v5M9 14.5v5",
  expand: "M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5",
  compress: "M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5",
  calendar: "M5 6h14v14H5zM5 10h14M9 3.5v4M15 3.5v4",
  copy: "M9 9h11v11H9zM5 15H4V4h11v1",
  close: "M6 6l12 12M18 6L6 18",
  share: "M12 15V4M8 8l4-4 4 4M8.5 11H6v9h12v-9h-2.5",
  chevron: "M8 10l4 4 4-4",
  check: "M5 12.5l4.5 4.5L19 7.5",
  search: "M11 18a7 7 0 1 1 0-14 7 7 0 0 1 0 14ZM20 20l-4-4",
} as const;

export type IconName = keyof typeof PATHS;

export function Icon({ name, size = 18 }: { name: IconName; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.75"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      style={{ display: "block", flex: "none" }}
    >
      <path d={PATHS[name]} />
    </svg>
  );
}
