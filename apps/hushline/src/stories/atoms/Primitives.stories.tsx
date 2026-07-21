import type { Meta, StoryObj } from "@storybook/react-vite";
import { Check, Download, Mic2 } from "lucide-react";
import { Button, Input, Select } from "@/shared/ui";

const meta = { title: "Atomic Design/Atoms/Primitives", parameters: { docs: { description: { component: "Hushline 화면을 구성하는 최소 단위 입력과 액션 컴포넌트입니다." } } } } satisfies Meta;
export default meta;
type Story = StoryObj<typeof meta>;

export const Buttons: Story = { render: () => <div className="flex flex-wrap items-center gap-3"><Button><Mic2 size={16}/>변환 시작</Button><Button variant="secondary"><Download size={16}/>모델 다운로드</Button><Button variant="ghost"><Check size={16}/>완료</Button><Button disabled>처리 중</Button></div> };
export const Inputs: Story = { render: () => <div className="grid max-w-xl gap-4"><Input defaultValue="https://youtube.com/watch?v=sample"/><Input placeholder="YouTube URL을 입력하세요"/><Select defaultValue="ko"><option value="auto">자동 감지</option><option value="ko">한국어</option><option value="en">English</option></Select></div> };
