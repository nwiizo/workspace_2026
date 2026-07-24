type InitialChair = {
  id: string;
  owner_id: string;
  name: string;
  model: string;
  token: string;
};

type InitialOwner = {
  id: string;
  name: string;
  token: string;
};

type InitialUser = {
  id: string;
  username: string;
  firstname: string;
  lastname: string;
  token: string;
  date_of_birth: string;
  invitation_code: string;
};

type initialDataType =
  | {
      owners: InitialOwner[];
      chair: InitialChair;
      users: InitialUser[];
    }
  | undefined;

const initialData = __INITIAL_DATA__ as initialDataType;

export const getOwners = (): InitialOwner[] => {
  return initialData?.owners ?? [];
};

export const getUsers = (): InitialUser[] => {
  return initialData?.users ?? [];
};

export const getSimulateChair = (): InitialChair | undefined => {
  return initialData?.chair;
};
