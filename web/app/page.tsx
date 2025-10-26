import { TopBar } from "@/components/layout/top-bar"
import { LeftSidebar } from "@/components/layout/left-sidebar"
import { MiddleColumn } from "@/components/layout/middle-column"
import { RightSidebar } from "@/components/layout/right-sidebar"

export default function HomePage() {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background">
      <TopBar />
      <div className="flex flex-1 overflow-hidden">
        <LeftSidebar />
        <MiddleColumn />
        <RightSidebar />
      </div>
    </div>
  )
}
