export interface NavItem {
  label: string;
  href: string;
}

export interface Surface {
  name: string;
  eyebrow: string;
  description: string;
  detail: string;
}

export interface Feature {
  title: string;
  description: string;
  status: 'stable' | 'supported' | 'early';
}

export interface FlowStep {
  label: string;
  title: string;
  description: string;
}

export interface UseCase {
  title: string;
  description: string;
}

export interface CommandGroup {
  label: string;
  commands: string[];
  note: string;
}

export const navItems: NavItem[] = [
  { label: '定位', href: '#why' },
  { label: '入口', href: '#surfaces' },
  { label: '架构', href: '#how-it-works' },
  { label: '快速开始', href: '#quickstart' },
];

export const surfaces: Surface[] = [
  {
    name: '桌面 GUI',
    eyebrow: 'Daily control',
    description: '面向日常使用的现代桌面控制台。',
    detail: '添加设备、创建路线、触发同步、开启监听、查看节点健康和最近活动都在一个界面中完成。',
  },
  {
    name: 'TUI Dashboard',
    eyebrow: 'Terminal native',
    description: '为服务器、远端机器和终端工作流保留完整控制感。',
    detail: '用键盘完成同步、监听、刷新、语言切换和任务检查，适合 SSH 与 Homelab 环境。',
  },
  {
    name: 'CLI',
    eyebrow: 'Scriptable routes',
    description: '节点、任务、同步、扫描、监听、日志都能脚本化。',
    detail: '把 TurboSync 接入开发脚本、远端机器、Docker 或 CI 辅助流程时，CLI 是最直接的入口。',
  },
];

export const features: Feature[] = [
  {
    title: '本地目录同步',
    description: '将源目录稳定同步到本机或挂载目标目录，适合工作目录与 NAS 备份。',
    status: 'stable',
  },
  {
    title: 'QUIC 远端传输',
    description: 'Agent-to-Agent 传输链路，面向服务器、工作站和 Homelab 设备。',
    status: 'supported',
  },
  {
    title: '500ms 文件监听防抖',
    description: '文件变化先合并事件，再扫描最终状态，避免把临时事件误当作真实同步计划。',
    status: 'supported',
  },
  {
    title: '同步日志与最近活动',
    description: '记录运行状态、变更数量、失败数量和传输数据量，让同步过程可观测。',
    status: 'supported',
  },
  {
    title: '节点健康检查',
    description: '持续查看连接成功、连接失败和错误信息，快速判断远端节点状态。',
    status: 'supported',
  },
  {
    title: '双向同步验证路径',
    description: '已有基础能力，适合在测试目录中验证；真实数据仍建议优先使用单向同步。',
    status: 'early',
  },
];

export const flowSteps: FlowStep[] = [
  {
    label: '01',
    title: 'GUI / TUI / CLI 发起控制',
    description: '所有入口都连接本机 Agent，不需要用户手动理解后台细节。',
  },
  {
    label: '02',
    title: 'Agent 扫描与监听源目录',
    description: 'walkdir、notify 和 Blake3 共同生成可靠的文件索引。',
  },
  {
    label: '03',
    title: 'SQLite 保存状态并生成计划',
    description: '同步引擎计算创建、更新、删除操作，以最终扫描状态为准。',
  },
  {
    label: '04',
    title: '本地复制或 QUIC 发送到目标 Agent',
    description: '目标可以是本机目录，也可以是可信网络中的远端节点。',
  },
];

export const useCases: UseCase[] = [
  {
    title: '开发目录跨设备',
    description: '把代码、笔记、配置和工作目录同步到另一台机器。',
  },
  {
    title: '工作站 → NAS',
    description: '不部署复杂平台，也能把本机目录持续同步到 NAS。',
  },
  {
    title: '服务器 / 远端机器',
    description: '用 Agent 和 CLI 在可信网络中管理远端同步路线。',
  },
  {
    title: 'Homelab 多节点',
    description: '在自有设备之间建立透明、可观测、可维护的同步链路。',
  },
];

export const commandGroups: CommandGroup[] = [
  {
    label: 'Desktop GUI',
    commands: ['make gui'],
    note: '自动构建并启动本机 Agent 与桌面控制台。',
  },
  {
    label: 'TUI Dashboard',
    commands: ['tsync dashboard'],
    note: '适合终端、SSH 和服务器操作习惯。',
  },
  {
    label: 'CLI route',
    commands: [
      'tsync agent run',
      'tsync node add nas 192.168.1.20:38746',
      'tsync task add documents --source ~/Documents --target-node <node-id> --target-path /backup/Documents',
      'tsync sync <task-id>',
    ],
    note: '用于远端机器、Docker、脚本化和自动化流程。',
  },
];

export const stabilityNotes = [
  '当前最稳定主线是单向同步：源目录 → 目标目录。',
  '双向同步已有基础验证路径，真实数据建议先在测试目录验证。',
  '远端同步建议运行在可信网络中，并使用证书指纹校验。',
  'TurboSync 不是云盘、协作平台或企业管理后台。',
];
