import type { PropsWithChildren, FC } from "react";

export const ErrorMessage: FC<PropsWithChildren> = ({ children }) => {
  return (
    <div className="flex justify-center items-center h-full w-full p-8">
      <p className="font-bold">{children}</p>
    </div>
  );
};
