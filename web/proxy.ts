import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";

export function proxy(request: NextRequest) {
  if (!request.nextUrl.searchParams.has("_ts")) {
    return NextResponse.next();
  }

  request.nextUrl.searchParams.delete("_ts");
  return NextResponse.next();
}

export const config = {
  matcher: ["/:path*"],
};
