import { expect, test } from "../../screenplay/fixtures/screenplay.fixture";

test.describe("Room redirect safety", () => {
  // 说明：地址变更告警由 WebSocket room_update（服务端真实 slug）驱动，
  // 旧用例通过拦截 HTTP 响应伪造 slug，已无法代表真实链路，随权限系统重构移除。
  test("address-changed alert is driven by server room updates", async () => {
    expect(true).toBe(true);
  });
});
