import type { ComponentProps, FC } from "react";
import { twMerge } from "tailwind-merge";
import { RatingStar } from "~/components/icon/rating-star";

type RatingProps = ComponentProps<"div"> & {
  size?: number;
  rating: number;
};

export const Rating: FC<RatingProps> = ({
  size = 40,
  rating,
  className,
  ...props
}) => {
  return (
    <div className={twMerge("flex flex-row gap-1", className)} {...props}>
      {Array.from({ length: 5 }).map((_, index) => {
        const starValue = index + 1;
        return (
          <RatingStar
            key={index}
            rated={starValue <= rating}
            width={size}
            height={size}
          />
        );
      })}
    </div>
  );
};
