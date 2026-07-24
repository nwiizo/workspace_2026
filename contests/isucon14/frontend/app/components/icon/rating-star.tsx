import { ComponentProps, FC } from "react";
import { twMerge } from "tailwind-merge";

export const RatingStar: FC<ComponentProps<"svg"> & { rated: boolean }> = ({
  rated,
  className,
  ...props
}) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill={rated ? "currentColor" : "#d9d9d9"}
      stroke={rated ? "currentColor" : "#d9d9d9"}
      className={twMerge(
        rated ? "text-yellow-400" : "text-neutral-300",
        className,
      )}
      {...props}
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="2"
        d="M12 17.27L18.18 21l-1.64-7.03L22 9.24l-7.19-.61L12 2 9.19 8.63 2 9.24l5.46 4.73L5.82 21z"
      />
    </svg>
  );
};
