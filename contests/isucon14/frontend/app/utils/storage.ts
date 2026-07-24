import { CampaignData, Coordinate } from "~/types";

const GroupId = "isuride";

const setStorage = (
  fieldId: string,
  itemData: number | string | { [key: string]: unknown } | undefined | null,
  storage: Storage,
): boolean => {
  try {
    const existing = JSON.parse(storage.getItem(GroupId) || "{}") as Record<
      string,
      string
    >;
    storage.setItem(
      GroupId,
      JSON.stringify({ ...existing, [fieldId]: itemData }),
    );
    return true;
  } catch (e) {
    return false;
  }
};

const getStorage = <T>(fieldId: string, storage: Storage): T | null => {
  try {
    const data = JSON.parse(storage.getItem(GroupId) || "{}") as Record<
      string,
      unknown
    >;
    return (data[fieldId] ?? null) as T | null;
  } catch (e) {
    return null;
  }
};

export const saveCampaignData = (campaign: CampaignData) => {
  return setStorage("campaign", campaign, localStorage);
};

export const getCampaignData = (): CampaignData | null => {
  return getStorage("campaign", localStorage);
};

export const setSimulatorCurrentCoordinate = (coordinate: Coordinate) => {
  return setStorage("simulator.currentCoordinate", coordinate, sessionStorage);
};

export const getSimulatorCurrentCoordinate = (): Coordinate | null => {
  return getStorage("simulator.currentCoordinate", sessionStorage);
};

export const setSimulatorStartCoordinate = (coordinate: Coordinate) => {
  return setStorage("simulator.startCoordinate", coordinate, sessionStorage);
};

export const getSimulatorStartCoordinate = (): Coordinate | null => {
  return getStorage("simulator.startCoordinate", sessionStorage);
};

export const setUserId = (id: string) => {
  return setStorage("user.id", id, sessionStorage);
};

export const getUserId = (): string | null => {
  return getStorage("user.id", sessionStorage);
};

export const setSimulatorCurrentRideId = (rideId: string) => {
  return setStorage("simulator.currentRideId", rideId, sessionStorage);
};

export const getSimulatorCurrentRideId = (): string | null => {
  return getStorage("simulator.currentRideId", sessionStorage);
};
