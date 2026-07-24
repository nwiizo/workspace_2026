import {
  ReactNode,
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import type { Coordinate, RideStatus } from "~/api/api-schemas";
import { getSimulateChair } from "~/utils/get-initial-data";

import { apiBaseURL } from "~/api/api-base-url";
import {
  ChairGetNotificationResponse,
  fetchChairGetNotification,
} from "~/api/api-components";
import { SimulatorChair } from "~/types";
import { Message, MessageTypes } from "~/utils/post-message";
import {
  getSimulatorCurrentCoordinate,
  getSimulatorCurrentRideId,
  setSimulatorCurrentRideId,
} from "~/utils/storage";

type SimulatorContextProps = {
  chair?: SimulatorChair;
  data?: ChairGetNotificationResponse["data"];
  setCoordinate?: (coordinate: Coordinate) => void;
  isAnotherSimulatorBeingUsed?: boolean;
};

const SimulatorContext = createContext<SimulatorContextProps>({});
const initilalChair = getSimulateChair();

function jsonFromSseResult<T>(value: string) {
  const data = value.slice("data:".length).trim();
  return JSON.parse(data) as T;
}

function isRiding(status: RideStatus | undefined) {
  return (
    status === "ARRIVED" ||
    status === "CARRYING" ||
    status === "ENROUTE" ||
    status === "PICKUP"
  );
}

const useNotification = (): ChairGetNotificationResponse["data"] => {
  const [isSse, setIsSse] = useState(false);
  const [notification, setNotification] =
    useState<ChairGetNotificationResponse>();

  useEffect(() => {
    const initialFetch = async () => {
      try {
        const notification = await fetch(`${apiBaseURL}/chair/notification`);
        const isEventStream = !!notification?.headers
          .get("Content-type")
          ?.split(";")?.[0]
          .includes("text/event-stream");
        setIsSse(isEventStream);

        if (isEventStream) {
          const reader = notification.body?.getReader();
          const decoder = new TextDecoder();
          const readed = (await reader?.read())?.value;
          const decoded = decoder.decode(readed);
          const json =
            jsonFromSseResult<ChairGetNotificationResponse["data"]>(decoded);
          setNotification(json ? { data: json } : undefined);
          return;
        }
        const json = (await notification.json()) as
          | ChairGetNotificationResponse
          | undefined;
        setNotification(json);
      } catch (error) {
        if (error instanceof DOMException && error.name === "AbortError") {
          return;
        }
        console.error(error);
      }
    };
    void initialFetch();
  }, [setNotification]);

  const retryAfterMs = notification?.retry_after_ms ?? 10000;

  useEffect(() => {
    if (!isSse) return;
    const eventSource = new EventSource(`${apiBaseURL}/chair/notification`);
    const onMessage = ({ data }: MessageEvent<{ data?: unknown }>) => {
      if (typeof data !== "string") return;
      try {
        const eventData = JSON.parse(
          data,
        ) as ChairGetNotificationResponse["data"];
        setNotification((preRequest) => {
          if (
            preRequest === undefined ||
            eventData?.status !== preRequest.data?.status ||
            eventData?.ride_id !== preRequest.data?.ride_id
          ) {
            return {
              data: eventData,
              contentType: "event-stream",
            };
          } else {
            return preRequest;
          }
        });
      } catch (error) {
        console.error(error);
      }
      return () => {
        eventSource.close();
      };
    };
    eventSource.addEventListener("message", onMessage);
    return () => {
      eventSource.close();
    };
  }, [isSse]);

  useEffect(() => {
    if (isSse) return;
    let timeoutId: ReturnType<typeof setTimeout>;
    let abortController: AbortController | undefined;

    const polling = async () => {
      try {
        abortController = new AbortController();
        const currentNotification = await fetchChairGetNotification(
          {},
          abortController.signal,
        );
        setNotification((preRequest) => {
          if (
            preRequest === undefined ||
            currentNotification?.data?.status !== preRequest.data?.status ||
            currentNotification?.data?.ride_id !== preRequest.data?.ride_id
          ) {
            return {
              data: currentNotification.data,
              retry_after_ms: currentNotification.retry_after_ms,
              contentType: "json",
            };
          } else {
            return preRequest;
          }
        });
        timeoutId = setTimeout(() => void polling(), retryAfterMs);
      } catch (error) {
        if (error instanceof DOMException && error.name === "AbortError") {
          return;
        }
        console.error(error);
      }
    };

    timeoutId = setTimeout(() => void polling(), retryAfterMs);

    return () => {
      abortController?.abort();
      clearTimeout(timeoutId);
    };
  }, [isSse, retryAfterMs]);

  return notification?.data;
};

export const SimulatorProvider = ({ children }: { children: ReactNode }) => {
  useEffect(() => {
    if (initilalChair?.token) {
      document.cookie = `chair_session=${initilalChair.token}; path=/`;
    }
  }, []);

  const data = useNotification();

  const [coordinate, setCoordinate] = useState<Coordinate>(() => {
    const coordinate = getSimulatorCurrentCoordinate();
    return coordinate ?? { latitude: 0, longitude: 0 };
  });

  const [clientRideId, setClientRideId] = useState<string>();
  useEffect(() => {
    setClientRideId((prev) => prev ?? getSimulatorCurrentRideId() ?? undefined);
  }, []);

  const isAnotherSimulatorBeingUsed = useMemo(() => {
    return isRiding(data?.status) && clientRideId !== data?.ride_id;
  }, [clientRideId, data]);

  useEffect(() => {
    const onMessage = ({
      data,
    }: MessageEvent<Message["ClientRideRequested"]>) => {
      const isSameOrigin = origin == location.origin;
      if (isSameOrigin && data.type === MessageTypes.ClientRideRequested) {
        const rideId = data?.payload?.rideId;
        if (rideId) {
          setClientRideId(rideId);
          setSimulatorCurrentRideId(rideId);
        }
      }
    };
    window.addEventListener("message", onMessage);
    return () => {
      window.removeEventListener("message", onMessage);
    };
  }, []);

  return (
    <SimulatorContext.Provider
      value={{
        data,
        chair: initilalChair ? { ...initilalChair, coordinate } : undefined,
        setCoordinate,
        isAnotherSimulatorBeingUsed,
      }}
    >
      {children}
    </SimulatorContext.Provider>
  );
};

export const useSimulatorContext = () => useContext(SimulatorContext);
