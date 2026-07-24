import { useNavigate } from "@remix-run/react";
import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import { apiBaseURL } from "~/api/api-base-url";
import {
  AppGetNotificationResponse,
  fetchAppGetNotification,
} from "~/api/api-components";
import { getCookieValue } from "~/utils/get-cookie-value";
import { getUserId } from "~/utils/storage";

function jsonFromSSEResponse<T>(value: string) {
  const data = value.slice("data:".length).trim();
  try {
    return JSON.parse(data) as T;
  } catch (e) {
    console.error(`don't parse ${value}`);
  }
}

export const useNotification = ():
  | AppGetNotificationResponse["data"]
  | undefined => {
  const navigate = useNavigate();
  const [isSse, setIsSse] = useState(false);
  const [notification, setNotification] =
    useState<AppGetNotificationResponse>();
  const retryAfterMs = notification?.retry_after_ms ?? 10000;

  useEffect(() => {
    const initialFetch = async () => {
      try {
        const notification = await fetch(`${apiBaseURL}/app/notification`);
        if (notification.status === 401) {
          navigate("/client/register");
          return;
        }
        const isEventStream = notification?.headers
          .get("Content-type")
          ?.split(";")[0]
          .includes("text/event-stream");
        setIsSse(!!isEventStream);
        if (isEventStream) {
          const reader = notification.body?.getReader();
          const decoder = new TextDecoder();
          const readed = (await reader?.read())?.value;
          const decoded = decoder.decode(readed);
          const json =
            jsonFromSSEResponse<AppGetNotificationResponse["data"]>(decoded);
          setNotification(json ? { data: json } : undefined);
          return;
        }
        const json = (await notification.json()) as
          | AppGetNotificationResponse
          | undefined;
        setNotification(json);
      } catch (error) {
        console.error(error);
      }
    };
    void initialFetch();
  }, [navigate]);

  useEffect(() => {
    if (!isSse) return;
    const eventSource = new EventSource(`${apiBaseURL}/app/notification`);
    eventSource.addEventListener("message", (event) => {
      if (typeof event.data === "string") {
        const eventData = JSON.parse(
          event.data,
        ) as AppGetNotificationResponse["data"];
        setNotification((prev) => {
          if (
            prev === undefined ||
            eventData?.status !== prev?.data?.status ||
            eventData?.ride_id !== prev?.data?.ride_id
          ) {
            return { data: eventData };
          } else {
            return prev;
          }
        });
      }
      return () => {
        eventSource.close();
      };
    });
  }, [isSse, setNotification]);

  useEffect(() => {
    if (isSse) return;

    let timeoutId: ReturnType<typeof setTimeout>;
    let abortController: AbortController | undefined;

    const polling = async () => {
      try {
        abortController = new AbortController();
        const currentNotification = await fetchAppGetNotification(
          {},
          abortController.signal,
        );
        setNotification((prev) => {
          if (
            prev?.data === undefined ||
            prev?.data?.status !== currentNotification.data?.status ||
            prev?.data?.ride_id !== currentNotification.data?.ride_id
          ) {
            return currentNotification;
          } else {
            return prev;
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
  }, [isSse, navigate, retryAfterMs]);

  return notification?.data;
};

type ClientContextProps = {
  data?: AppGetNotificationResponse["data"];
  userId?: string | null;
};

const ClientContext = createContext<ClientContextProps>({});

export const ClientProvider = ({ children }: { children: ReactNode }) => {
  const [userId] = useState(() => getUserId());
  const navigate = useNavigate();
  const data = useNotification();

  useEffect(() => {
    const isRegistered =
      typeof getCookieValue(document.cookie, "app_session") !== "undefined";
    if (!isRegistered) {
      navigate("/client/register");
    }
  }, [navigate]);

  return (
    <ClientContext.Provider value={{ data, userId }}>
      {children}
    </ClientContext.Provider>
  );
};

export const useClientContext = () => useContext(ClientContext);
