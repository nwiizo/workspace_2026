import { Outlet } from "@remix-run/react";
import { FooterNavigation } from "~/components/modules/footer-navigation/footer-navigation";
import { MainFrame } from "~/components/primitives/frame/frame";
import { Text } from "~/components/primitives/text/text";
import { ClientProvider } from "../../contexts/client-context";

export default function ClientLayout() {
  return (
    <MainFrame>
      <Text tagName="h1" className="sr-only">
        ISURIDE
      </Text>
      <ClientProvider>
        <Outlet />
      </ClientProvider>
      <FooterNavigation />
    </MainFrame>
  );
}
