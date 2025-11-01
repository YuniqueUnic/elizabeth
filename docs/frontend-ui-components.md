# Elizabeth 项目前端 UI 组件库详细文档

## 模块概述

UI 组件库是 Elizabeth 项目的基础用户界面组件集合，基于 shadcn/ui
组件库构建。该模块提供了丰富的 UI
组件，包括按钮、输入框、卡片、对话框等，为整个应用提供了一致的设计语言和交互体验。

## 功能描述

UI 组件库主要负责：

- 提供基础 UI 组件的封装和扩展
- 统一的设计语言和交互模式
- 支持主题系统和响应式设计
- 提供可访问性支持和键盘导航
- 集成表单验证和错误处理机制

## 组件清单

### 1. Button 组件

**文件位置**: `web/components/ui/button.tsx`

**功能描述**: 基础按钮组件，支持多种变体和尺寸

**核心特性**:

- **多种变体**: default、destructive、outline、secondary、ghost、link
- **尺寸选项**: sm、md、lg、icon
- **加载状态**: 支持 loading 状态和禁用
- **图标支持**: 集成 Lucide React 图标库

### 2. Input 组件

**文件位置**: `web/components/ui/input.tsx`

**功能描述**: 基础输入框组件，支持多种输入类型和验证

**核心特性**:

- **输入类型**: text、email、password、number、tel、url、search、file
- **验证支持**: 内置验证规则和自定义验证函数
- **标签和占位符**: 支持标签、前缀、后缀和占位符
- **错误处理**: 错误状态显示和错误消息

### 3. Card 组件

**文件位置**: `web/components/ui/card.tsx`

**功能描述**: 卡片容器组件，支持阴影、边框和内容布局

**核心特性**:

- **阴影效果**: 支持多种阴影样式
- **边框选项**: 支持边框、圆角和分割线
- **内容布局**: 灵活的 header、content、footer 布局
- **响应式设计**: 移动端和桌面端自适应

### 4. Dialog 组件

**文件位置**: `web/components/ui/dialog.tsx`

**功能描述**: 模态对话框组件，支持多种对话框类型和交互

**核心特性**:

- **对话框类型**: alert、confirm、prompt、sheet、drawer
- **关闭机制**: 支持点击外部关闭和 ESC 键关闭
- **动画效果**: 支持淡入淡出和滑动动画
- **焦点管理**: 自动焦点管理和键盘导航

### 5. Badge 组件

**文件位置**: `web/components/ui/badge.tsx`

**功能描述**: 徽章组件，用于显示状态和计数

**核心特性**:

- **多种变体**: default、secondary、destructive、outline
- **状态显示**: 支持不同状态的视觉样式
- **内容支持**: 支持图标、文本和自定义内容
- **响应式设计**: 自适应尺寸和颜色

### 6. Avatar 组件

**文件位置**: `web/components/ui/avatar.tsx`

**功能描述**: 头像组件，支持图片、字母和状态显示

**核心特性**:

- **图片支持**: 支持多种图片格式和回退
- **字母头像**: 自动生成首字母头像
- **状态指示**: 支持在线、离线、忙碌等状态
- **尺寸选项**: sm、md、lg、xl、2xl

### 7. Separator 组件

**文件位置**: `web/components/ui/separator.tsx`

**功能描述**: 分隔线组件，用于视觉分组和布局

**核心特性**:

- **方向**: 水平、垂直
- **样式**: 支持自定义颜色、粗细和长度
- **装饰**: 支持文字装饰（点线、波浪线）

### 8. Table 组件

**文件位置**: `web/components/ui/table.tsx`

**功能描述**: 数据表格组件，支持排序、筛选和分页

**核心特性**:

- **数据源**: 支持数组和对象数组
- **列定义**: 灵活的列配置和渲染
- **排序**: 支持多列排序
- **分页**: 内置分页控件
- **选择**: 支持行选择和多选

### 9. Tabs 组件

**文件位置**: `web/components/ui/tabs.tsx`

**功能描述**: 标签页组件，支持动态标签内容和切换

**核心特性**:

- **动态标签**: 支持运行时添加和删除标签
- **标签内容**: 支持自定义标签内容和组件
- **切换动画**: 支持平滑的标签切换动画
- **滚动**: 支持标签滚动和自动激活

### 10. Toast 组件

**文件位置**: `web/components/ui/toast.tsx`

**功能描述**: 通知组件，用于显示操作反馈和状态信息

**核心特性**:

- **多种类型**: success、error、warning、info
- **位置选项**: 支持四个角落和中心位置
- **自动消失**: 支持自动和手动关闭
- **堆叠**: 支持多个通知的堆叠显示

## 实现细节

### 基础组件架构

#### 组件设计模式

```typescript
// 基础 Button 组件
interface ButtonProps {
  variant?: 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | 'link';
  size?: 'sm' | 'md' | 'lg';
  disabled?: boolean;
  loading?: boolean;
  children: React.ReactNode;
  onClick?: () => void;
  className?: string;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = 'default', size = 'md', ...props }, ref) => {
    const baseClasses = 'inline-flex items-center justify-center rounded-md font-medium transition-colors';
    const variantClasses = {
      default: 'bg-blue-600 text-white hover:bg-blue-700',
      destructive: 'bg-red-600 text-white hover:bg-red-700',
      outline: 'border border-gray-300 bg-white text-gray-700 hover:bg-gray-50',
      secondary: 'bg-gray-100 text-gray-900 hover:bg-gray-200',
      ghost: 'border border-gray-300 bg-transparent text-gray-700 hover:bg-gray-100',
      link: 'text-blue-600 underline hover:text-blue-800',
    };

    const sizeClasses = {
      sm: 'px-3 py-1.5 text-sm',
      md: 'px-4 py-2 text-base',
      lg: 'px-6 py-3 text-lg',
    };

    return (
      <button
        className={cn(baseClasses, variantClasses[variant], sizeClasses[size], className, {
          'opacity-50 cursor-not-allowed': disabled,
        'pointer-events-none': loading,
        })}
        ref={ref}
        {...props}
      >
        {loading ? <LoadingSpinner size="sm" /> : children}
      </button>
    );
  );
});
```

#### 样式系统集成

- **Tailwind CSS**: 使用 utility classes 进行样式管理
- **主题支持**: 通过 CSS 变量支持主题切换
- **响应式设计**: 支持不同屏幕尺寸的适配

### 高级组件特性

#### 动画系统

```typescript
// 使用 Framer Motion 实现动画
import { AnimatePresence, motion } from "framer-motion";

export const AnimatedCard = (
  { children, isVisible }: { children: React.ReactNode; isVisible: boolean },
) => (
  <AnimatePresence>
    {isVisible
      ? (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -20 }}
          transition={{ duration: 0.3 }}
        >
          {children}
        </motion.div>
      )
      : null}
  </AnimatePresence>
);
```

#### 表单集成

```typescript
// 表单验证集成
import { useForm } from 'react-hook-form';

export const ValidatedInput = ({ name, validation, onSubmit }: {
  name: string;
  validation: {
    required: true,
    minLength: 3,
    pattern: /^[a-zA-Z0-9]+$/,
  };

  const { register, errors } = useForm({
    defaultValues: { [name]: '' },
    resolver: async (data) => {
      const result = validation.safeParse(data);
      if (result.success) {
        await onSubmit(result.data);
        return { values: result.data, errors: {} };
      }
      return { values: {}, errors: result.error };
    },
  });

  return (
    <form onSubmit={register}>
      <input name={name} />
      {errors.name && <span className="text-red-500">{errors.name}</span>}
      <button type="submit">提交</button>
    </form>
  );
};
```

## 组件间交互

### 事件处理

- **事件传播**: 通过 props 传递用户操作和状态变更
- **回调机制**: 标准化的回调函数接口
- **状态同步**: 通过共享状态管理实现组件间协调

### 与状态管理集成

- **全局状态**: 通过 Zustand store 访问全局状态
- **状态更新**: 通过 setter 函数更新全局状态
- **主题支持**: 组件样式与主题系统同步

## 使用示例

### Button 组件使用示例

```tsx
import { Button } from "@/components/ui/button";

export default function ExampleUsage() {
  return (
    <div className="space-y-4">
      <Button onClick={() => console.log("Default clicked")}>
        默认按钮
      </Button>

      <Button
        variant="destructive"
        onClick={() => console.log("Destructive clicked")}
      >
        危险按钮
      </Button>

      <Button
        variant="outline"
        size="lg"
        onClick={() => console.log("Large outline clicked")}
      >
        大尺寸轮廓按钮
      </Button>

      <Button disabled loading onClick={() => console.log("Loading clicked")}>
        加载中按钮
      </Button>
    </div>
  );
}
```

### Card 组件使用示例

```tsx
import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
} from "@/components/ui/card";

export default function UserProfile({ user }) {
  return (
    <Card className="w-96">
      <CardHeader>
        <Avatar src={user.avatar} size="lg" />
        <div>
          <h3 className="text-lg font-semibold">{user.name}</h3>
          <p className="text-gray-600">{user.email}</p>
        </div>
      </CardHeader>

      <CardContent>
        <p>{user.bio}</p>
      </CardContent>

      <CardFooter>
        <Button size="sm">编辑资料</Button>
      </CardFooter>
    </Card>
  );
}
```

## 最佳实践

### 开发建议

1. **组件复用**: 充分利用组件的可复用性，避免重复代码
2. **状态管理**: 合理使用本地状态和全局状态，避免不必要的重渲染
3. **错误处理**: 实现完善的错误边界和错误处理机制
4. **性能优化**: 使用 React.memo 和 useMemo 优化渲染性能
5. **类型安全**: 充分利用 TypeScript 的类型检查，确保类型安全

### 使用注意事项

1. **可访问性**: 遵循 WCAG 无障碍访问标准，支持键盘导航
2. **响应式设计**: 确保组件在不同设备上都有良好的显示效果
3. **主题支持**: 组件样式与主题系统保持一致
4. **国际化**: 支持多语言和本地化文本

### 扩展指南

1. **新组件开发**: 遵循 shadcn/ui 的设计模式开发新组件
2. **自定义主题**: 支持扩展主题系统和自定义样式
3. **插件系统**: 考虑添加插件系统支持功能扩展

## 技术细节

### 技术栈

- **React 18**: 使用现代 React Hooks 和并发特性
- **TypeScript**: 完整的类型安全支持
- **Tailwind CSS**: 实用优先的 CSS 框架
- **Framer Motion**: 动画库支持
- **React Hook Form**: 表单验证库

### 依赖关系

```
UI组件库
├── button.tsx (按钮)
├── input.tsx (输入框)
├── card.tsx (卡片)
├── dialog.tsx (对话框)
├── badge.tsx (徽章)
├── avatar.tsx (头像)
├── separator.tsx (分隔线)
├── table.tsx (表格)
├── tabs.tsx (标签页)
├── toast.tsx (通知)
└── index.ts (导出)
```

### 性能特点

- **虚拟化**: 支持大量数据的高效渲染
- **懒加载**: 按需加载组件和资源
- **缓存**: 组件级别的缓存策略
- **代码分割**: 自动代码分割和懒加载

## 架构优势

1. **设计系统**: 基于 shadcn/ui 的统一设计语言
2. **类型安全**: 完整的 TypeScript 类型支持
3. **可维护性**: 良好的代码组织和文档
4. **可扩展性**: 支持主题扩展和自定义样式
5. **用户体验**: 响应式设计和无障碍访问支持

该 UI 组件库为 Elizabeth
项目提供了现代化、一致、用户友好的界面组件解决方案，具备良好的可维护性和扩展性。
