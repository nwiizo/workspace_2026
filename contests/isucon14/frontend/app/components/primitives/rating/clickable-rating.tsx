import { ComponentProps, FC } from "react";
import { twMerge } from "tailwind-merge";
import { RatingStar } from "~/components/icon/rating-star";

type ClickableRatingProps = ComponentProps<"div"> & {
  name: string;
  starSize?: number;
  rating: number;
  setRating: React.Dispatch<React.SetStateAction<number>>;
};

export const ClickableRating: FC<ClickableRatingProps> = ({
  name,
  starSize = 40,
  rating,
  setRating,
  className,
  ...props
}) => {
  return (
    <div className={twMerge("flex flex-row gap-1", className)} {...props}>
      {Array.from({ length: 5 }).map((_, index) => {
        const starValue = index + 1;
        return (
          <label
            key={index}
            htmlFor={`${name}-${starValue}`}
            className="cursor-pointer flex items-center"
          >
            <input
              type="radio"
              id={`${name}-${starValue}`}
              name={name}
              value={starValue}
              aria-label={`Rating ${starValue}`}
              onClick={() => setRating(starValue)}
              className="hidden"
            />
            <RatingStar
              rated={starValue <= rating}
              onClick={() => setRating(starValue)}
              width={starSize}
              height={starSize}
            />
          </label>
        );
      })}
      <input type="hidden" name={name} value={rating} />
    </div>
  );
};
