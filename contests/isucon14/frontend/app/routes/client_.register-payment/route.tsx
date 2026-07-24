import type { MetaFunction } from "@remix-run/node";
import { ClientActionFunctionArgs, Form, redirect } from "@remix-run/react";
import { fetchAppPostPaymentMethods } from "~/api/api-components";
import { Button } from "~/components/primitives/button/button";
import { TextInput } from "~/components/primitives/form/text-input";
import { FormFrame } from "~/components/primitives/frame/form-frame";

export const meta: MetaFunction = () => {
  return [
    { title: "Regiter Payment | ISURIDE" },
    { name: "description", content: "決済トークン登録" },
  ];
};

export const clientAction = async ({ request }: ClientActionFunctionArgs) => {
  const formData = await request.formData();
  await fetchAppPostPaymentMethods({
    body: {
      token: String(formData.get("payment-token")),
    },
  });
  return redirect(`/client`);
};

export default function ClientRegister() {
  return (
    <FormFrame>
      <h1 className="text-2xl font-semibold mb-8">決済トークン登録</h1>
      <Form className="flex flex-col gap-8 w-full" method="POST">
        <div>
          <TextInput
            id="payment-token"
            name="payment-token"
            label="決済トークン"
            required
          />
        </div>
        <Button type="submit" variant="primary" className="text-lg mt-6">
          登録
        </Button>
      </Form>
    </FormFrame>
  );
}
