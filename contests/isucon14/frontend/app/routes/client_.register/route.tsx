import type { MetaFunction } from "@remix-run/node";
import {
  ClientActionFunctionArgs,
  Form,
  json,
  Link,
  redirect,
  useActionData,
} from "@remix-run/react";
import { fetchAppPostUsers } from "~/api/api-components";
import { Button } from "~/components/primitives/button/button";
import { DateInput } from "~/components/primitives/form/date-input";
import { TextInput } from "~/components/primitives/form/text-input";
import { FormFrame } from "~/components/primitives/frame/form-frame";
import { Text } from "~/components/primitives/text/text";
import { saveCampaignData } from "~/utils/storage";

export const meta: MetaFunction = () => {
  return [
    { title: "Regiter | ISURIDE" },
    { name: "description", content: "ユーザー登録" },
  ];
};

export const clientAction = async ({ request }: ClientActionFunctionArgs) => {
  const formData = await request.formData();

  // client validation
  const errors: {
    username?: string;
    firstname?: string;
    lastname?: string;
    register?: string;
    invitation_code?: string;
  } = {};

  const date_of_birth = String(formData.get("date_of_birth"));
  const username = String(formData.get("username"));
  const firstname = String(formData.get("firstname"));
  const lastname = String(formData.get("lastname"));
  const invitation_code = String(formData.get("invitation_code"));

  if (username.length > 30) {
    errors.username = "30文字以内で入力してください";
  }
  if (firstname.length > 30) {
    errors.firstname = "30文字以内で入力してください";
  }
  if (lastname.length > 30) {
    errors.lastname = "30文字以内で入力してください";
  }
  if (Object.keys(errors).length > 0) {
    return json({ errors });
  }

  try {
    const res = await fetchAppPostUsers({
      body: {
        date_of_birth: date_of_birth,
        username: username,
        firstname: firstname,
        lastname: lastname,
        invitation_code: invitation_code,
      },
    });

    saveCampaignData({
      invitationCode: res.invitation_code,
      registedAt: new Date().toISOString(),
      used: false,
    });

    return redirect(`/client/register-payment`);
  } catch (e) {
    console.error(`ERROR: ${JSON.stringify(e)}`);
    errors.register =
      "ユーザーの登録に失敗しました。接続に問題があるか、ユーザー名が登録済みの可能性があります。";
    return json({ errors });
  }
};

export default function ClientRegister() {
  const actionData = useActionData<typeof clientAction>();

  return (
    <FormFrame>
      <h1 className="text-2xl font-semibold mb-6">ユーザー登録</h1>
      {actionData?.errors?.register && (
        <Text variant="danger" size="sm" className="mb-6">
          {actionData?.errors?.register}
        </Text>
      )}
      <Form className="flex flex-col gap-4" method="POST">
        <div>
          <TextInput
            id="username"
            name="username"
            label="ユーザー名"
            required
          />
          {actionData?.errors?.username && (
            <Text variant="danger" className="mt-2">
              {actionData?.errors?.username}
            </Text>
          )}
        </div>
        <div className="flex gap-4">
          <div className="w-full">
            <TextInput id="lastname" name="lastname" label="姓" required />
            {actionData?.errors?.lastname && (
              <Text variant="danger" className="mt-2">
                {actionData?.errors?.lastname}
              </Text>
            )}
          </div>
          <div className="w-full">
            <TextInput id="firstname" name="firstname" label="名" required />
            {actionData?.errors?.firstname && (
              <Text variant="danger" className="mt-2">
                {actionData?.errors?.firstname}
              </Text>
            )}
          </div>
        </div>
        <div>
          <DateInput
            id="date_of_birth"
            name="date_of_birth"
            label="誕生日"
            defaultValue="2000-04-01"
            required
          />
        </div>
        <div className="mb-6">
          <TextInput
            id="invitation_code"
            name="invitation_code"
            label="招待コード (お持ちの方のみ入力)"
          />
          {actionData?.errors?.invitation_code && (
            <Text variant="danger" className="mt-2">
              {actionData?.errors?.invitation_code}
            </Text>
          )}
        </div>
        <Button type="submit" variant="primary" className="text-lg">
          登録
        </Button>
        <p className="text-center mt-2">
          <Link to="/client/login" className="text-blue-600 hover:underline">
            ログイン
          </Link>
        </p>
      </Form>
    </FormFrame>
  );
}
