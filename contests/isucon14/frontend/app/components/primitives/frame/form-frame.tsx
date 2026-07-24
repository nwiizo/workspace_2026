import type { FC, PropsWithChildren } from "react";

export const FormFrame: FC<PropsWithChildren> = ({ children }) => {
  return (
    <div className="sm:flex items-center justify-center h-screen bg-white sm:bg-inherit sm:mx-auto">
      <div className="bg-white sm:w-[500px] sm:rounded-md sm:shadow-md p-12">
        {children}
      </div>
    </div>
  );
};
