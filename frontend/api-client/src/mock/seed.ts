/**
 * Mock 种子数据：一套连贯的节点/实例/用户/权限样例，驱动前端原型。
 * 数据形状与 PLAN.md 第 6 节领域对象一致。
 */

import type {
  AuditEvent,
  ConfigDocument,
  ConfigField,
  CoreNode,
  CpuPolicy,
  CpuTopology,
  DeviceSession,
  ExtensionInstall,
  ExtensionProject,
  FileEntry,
  Group,
  ImageBuild,
  ImageInfo,
  InstallTemplate,
  Instance,
  InstanceRuntime,
  ManagedRuntime,
  Schedule,
  ScheduleExecution,
  Task,
  User,
} from '../domain';
import { ALL_PERMISSIONS } from '../domain';

export interface MockState {
  users: User[];
  groups: Group[];
  sessions: DeviceSession[];
  /** 用户名 → 明文密码。仅 Mock 使用；真实后端为 Argon2id 哈希（PLAN 4.3）。 */
  credentials: Record<string, string>;
  cores: CoreNode[];
  instances: Instance[];
  runtimes: Map<string, InstanceRuntime>;
  managedRuntimes: ManagedRuntime[];
  templates: InstallTemplate[];
  /** `${instanceId}:${dirPath}` → 目录内容。 */
  fileTree: Map<string, FileEntry[]>;
  /** `${instanceId}:${filePath}` → 文本内容。 */
  fileContents: Map<string, string>;
  configs: ConfigDocument[];
  configFields: Map<string, ConfigField[]>;
  configRaw: Map<string, string>;
  extensions: ExtensionInstall[];
  extensionCatalog: ExtensionProject[];
  images: ImageInfo[];
  builds: ImageBuild[];
  cpuTopologies: CpuTopology[];
  cpuPolicies: Map<string, CpuPolicy>;
  schedules: Schedule[];
  executions: ScheduleExecution[];
  tasks: Task[];
  audit: AuditEvent[];
}

const HOUR = 3_600_000;
const DAY = 24 * HOUR;

export function buildSeed(now = Date.now()): MockState {
  const cores: CoreNode[] = [
    {
      id: 'core-1',
      name: '上海节点-01',
      address: '192.168.10.11:24444',
      status: 'ONLINE',
      version: '0.1.0',
      os: 'linux',
      arch: 'x86_64',
      capabilities: ['docker', 'cpu-topology', 'mcdr'],
      lastSeenAt: now - 5_000,
      instanceCount: 3,
      runningCount: 2,
      cpuUsage: 0.42,
      memoryUsage: 0.63,
      createdAt: now - 90 * DAY,
    },
    {
      id: 'core-2',
      name: '备用节点-家用机',
      address: '192.168.1.20:24444',
      status: 'DEGRADED',
      version: '0.1.0',
      os: 'windows',
      arch: 'x86_64',
      capabilities: ['cpu-topology'],
      lastSeenAt: now - 60_000,
      instanceCount: 2,
      runningCount: 0,
      cpuUsage: 0.11,
      memoryUsage: 0.38,
      createdAt: now - 30 * DAY,
    },
  ];

  const instances: Instance[] = [
    {
      id: 'inst-1',
      coreId: 'core-1',
      name: '生存主服',
      serverType: 'PAPER',
      version: '1.21.4',
      state: 'RUNNING',
      workDir: '/opt/mc/servers/survival',
      javaRuntimeId: 'rt-java21',
      runtimeMode: 'HOST',
      containerImage: null,
      containerPorts: [],
      containerEnv: {},
      supervisorMode: 'MCDR',
      launchCommand: 'java -Xms4G -Xmx4G -jar server.jar nogui',
      updateCommand: null,
      expiresAt: null,
      tags: ['production', 'survival'],
      createdAt: now - 80 * DAY,
    },
    {
      id: 'inst-2',
      coreId: 'core-1',
      name: '创造测试服',
      serverType: 'VANILLA',
      version: '1.21.4',
      state: 'STOPPED',
      workDir: '/opt/mc/servers/creative-test',
      javaRuntimeId: 'rt-java21',
      runtimeMode: 'HOST',
      containerImage: null,
      containerPorts: [],
      containerEnv: {},
      supervisorMode: 'DIRECT',
      launchCommand: 'java -Xmx2G -jar server.jar nogui',
      updateCommand: null,
      expiresAt: now + 14 * DAY,
      tags: ['test'],
      createdAt: now - 20 * DAY,
    },
    {
      id: 'inst-3',
      coreId: 'core-1',
      name: '大厅代理',
      serverType: 'VELOCITY',
      version: '3.4.0',
      state: 'RUNNING',
      workDir: '/opt/mc/servers/lobby-proxy',
      javaRuntimeId: 'rt-java17',
      runtimeMode: 'HOST',
      containerImage: null,
      containerPorts: [],
      containerEnv: {},
      supervisorMode: 'DIRECT',
      launchCommand: 'java -Xmx1G -jar velocity.jar',
      updateCommand: null,
      expiresAt: null,
      tags: ['production', 'proxy'],
      createdAt: now - 75 * DAY,
    },
    {
      id: 'inst-4',
      coreId: 'core-2',
      name: '模组 Fabric 服',
      serverType: 'FABRIC',
      version: '1.20.1',
      state: 'FAILED',
      workDir: 'D:\\mc\\fabric-modded',
      javaRuntimeId: 'rt-java17-w2',
      runtimeMode: 'HOST',
      containerImage: null,
      containerPorts: [],
      containerEnv: {},
      supervisorMode: 'MCDR',
      launchCommand: 'java -Xmx6G -jar fabric-server.jar nogui',
      updateCommand: null,
      expiresAt: null,
      tags: ['modded'],
      createdAt: now - 10 * DAY,
    },
    {
      id: 'inst-5',
      coreId: 'core-2',
      name: '周末活动服',
      serverType: 'PAPER',
      version: '1.21.1',
      state: 'CREATED',
      workDir: 'D:\\mc\\weekend-event',
      javaRuntimeId: null,
      runtimeMode: 'HOST',
      containerImage: null,
      containerPorts: [],
      containerEnv: {},
      supervisorMode: 'DIRECT',
      launchCommand: 'java -Xmx2G -jar server.jar nogui',
      updateCommand: null,
      expiresAt: now + 7 * DAY,
      tags: ['event'],
      createdAt: now - 2 * DAY,
    },
  ];

  const runtimes = new Map<string, InstanceRuntime>([
    [
      'inst-1',
      {
        instanceId: 'inst-1',
        pid: 21834,
        state: 'RUNNING',
        startedAt: now - 26 * HOUR,
        exitCode: null,
        cpuUsage: 0.31,
        memoryBytes: 3_400_000_000,
        playerCount: 17,
      },
    ],
    [
      'inst-2',
      {
        instanceId: 'inst-2',
        pid: null,
        state: 'STOPPED',
        startedAt: null,
        exitCode: 0,
        cpuUsage: 0,
        memoryBytes: 0,
        playerCount: 0,
      },
    ],
    [
      'inst-3',
      {
        instanceId: 'inst-3',
        pid: 21901,
        state: 'RUNNING',
        startedAt: now - 26 * HOUR,
        exitCode: null,
        cpuUsage: 0.04,
        memoryBytes: 620_000_000,
        playerCount: 17,
      },
    ],
    [
      'inst-4',
      {
        instanceId: 'inst-4',
        pid: null,
        state: 'FAILED',
        startedAt: now - 3 * DAY,
        exitCode: 1,
        cpuUsage: 0,
        memoryBytes: 0,
        playerCount: null,
      },
    ],
    [
      'inst-5',
      {
        instanceId: 'inst-5',
        pid: null,
        state: 'CREATED',
        startedAt: null,
        exitCode: null,
        cpuUsage: 0,
        memoryBytes: 0,
        playerCount: null,
      },
    ],
  ]);

  const managedRuntimes: ManagedRuntime[] = [
    { id: 'rt-java21', coreId: 'core-1', kind: 'JAVA', version: '21.0.5+11', majorVersion: 21, path: '/opt/mcnp/runtimes/java/21.0.5', sha256: 'a1b2c3', installedAt: now - 70 * DAY, sizeBytes: 196_000_000 },
    { id: 'rt-java17', coreId: 'core-1', kind: 'JAVA', version: '17.0.13+11', majorVersion: 17, path: '/opt/mcnp/runtimes/java/17.0.13', sha256: 'd4e5f6', installedAt: now - 70 * DAY, sizeBytes: 188_000_000 },
    { id: 'rt-java8', coreId: 'core-1', kind: 'JAVA', version: '1.8.0_432', majorVersion: 8, path: '/opt/mcnp/runtimes/java/8u432', sha256: null, installedAt: now - 60 * DAY, sizeBytes: 172_000_000 },
    { id: 'rt-node20', coreId: 'core-1', kind: 'NODE', version: '20.18.1', majorVersion: 20, path: '/opt/mcnp/runtimes/node/20.18.1', sha256: 'f6e5d4', installedAt: now - 40 * DAY, sizeBytes: 94_000_000 },
    { id: 'rt-py312', coreId: 'core-1', kind: 'PYTHON', version: '3.12.8', majorVersion: 3, path: '/opt/mcnp/runtimes/python/3.12.8', sha256: null, installedAt: now - 40 * DAY, sizeBytes: 128_000_000 },
    { id: 'rt-java21-w2', coreId: 'core-2', kind: 'JAVA', version: '21.0.5+11', majorVersion: 21, path: 'D:\\mcnp\\runtimes\\java\\21.0.5', sha256: 'a1b2c3', installedAt: now - 25 * DAY, sizeBytes: 196_000_000 },
    { id: 'rt-java17-w2', coreId: 'core-2', kind: 'JAVA', version: '17.0.13+11', majorVersion: 17, path: 'D:\\mcnp\\runtimes\\java\\17.0.13', sha256: 'd4e5f6', installedAt: now - 25 * DAY, sizeBytes: 188_000_000 },
  ];

  const templates: InstallTemplate[] = [
    {
      id: 'tpl-vanilla',
      name: 'Vanilla 原版',
      serverType: 'VANILLA',
      description: 'Mojang 官方服务端，无任何修改。',
      versions: ['1.21.4', '1.21.1', '1.20.6', '1.20.1'],
      requiredRuntime: { kind: 'JAVA', minMajor: 21 },
      supportsMcdr: true,
    },
    {
      id: 'tpl-paper',
      name: 'Paper',
      serverType: 'PAPER',
      description: '高性能 Bukkit 兼容服务端，插件生态丰富。',
      versions: ['1.21.4', '1.21.1', '1.20.6', '1.20.4', '1.20.1'],
      requiredRuntime: { kind: 'JAVA', minMajor: 21 },
      supportsMcdr: true,
    },
    {
      id: 'tpl-velocity',
      name: 'Velocity',
      serverType: 'VELOCITY',
      description: '现代高性能代理端，用于多服组网。',
      versions: ['3.4.0', '3.3.0'],
      requiredRuntime: { kind: 'JAVA', minMajor: 17 },
      supportsMcdr: false,
    },
    {
      id: 'tpl-fabric',
      name: 'Fabric',
      serverType: 'FABRIC',
      description: '轻量模组加载器，适合模组服。',
      versions: ['1.21.4', '1.21.1', '1.20.6', '1.20.1'],
      requiredRuntime: { kind: 'JAVA', minMajor: 21 },
      supportsMcdr: true,
    },
  ];

  const fileTree = new Map<string, FileEntry[]>([
    [
      'inst-1:/',
      [
        { name: 'plugins', path: '/plugins', isDir: true, sizeBytes: 0, modifiedAt: now - 2 * DAY },
        { name: 'config', path: '/config', isDir: true, sizeBytes: 0, modifiedAt: now - 2 * DAY },
        { name: 'world', path: '/world', isDir: true, sizeBytes: 0, modifiedAt: now - HOUR },
        { name: 'logs', path: '/logs', isDir: true, sizeBytes: 0, modifiedAt: now - 600_000 },
        { name: 'server.properties', path: '/server.properties', isDir: false, sizeBytes: 1_312, modifiedAt: now - 2 * DAY },
        { name: 'paper-global.yml', path: '/paper-global.yml', isDir: false, sizeBytes: 2_048, modifiedAt: now - 2 * DAY },
        { name: 'eula.txt', path: '/eula.txt', isDir: false, sizeBytes: 182, modifiedAt: now - 80 * DAY },
      ],
    ],
    [
      'inst-1:/plugins',
      [
        { name: 'LuckPerms-Bukkit-5.4.141.jar', path: '/plugins/LuckPerms-Bukkit-5.4.141.jar', isDir: false, sizeBytes: 7_200_000, modifiedAt: now - 20 * DAY },
        { name: 'Chunky-Bukkit-1.4.28.jar', path: '/plugins/Chunky-Bukkit-1.4.28.jar', isDir: false, sizeBytes: 1_900_000, modifiedAt: now - 20 * DAY },
        { name: 'spark-1.10.109-bukkit.jar', path: '/plugins/spark-1.10.109-bukkit.jar', isDir: false, sizeBytes: 8_800_000, modifiedAt: now - 20 * DAY },
      ],
    ],
    [
      'inst-1:/logs',
      [
        { name: 'latest.log', path: '/logs/latest.log', isDir: false, sizeBytes: 1_420_000, modifiedAt: now - 600_000 },
        { name: '2024-12-01-1.log.gz', path: '/logs/2024-12-01-1.log.gz', isDir: false, sizeBytes: 320_000, modifiedAt: now - 9 * DAY },
      ],
    ],
  ]);

  const serverProperties = [
    '#Minecraft server properties',
    'motd=MCNP 生存主服',
    'server-port=25565',
    'max-players=50',
    'online-mode=true',
    'difficulty=normal',
    'view-distance=10',
    'simulation-distance=8',
    'spawn-protection=16',
    'enable-command-block=false',
    'white-list=false',
    'pvp=true',
  ].join('\n');

  const paperGlobal = [
    'chunk-loading:',
    '  autoconfig-send-distance: true',
    '  enable-frustum-priority: false',
    'misc:',
    '  fix-entity-position-desync: true',
    '  lag-compensate-block-breaking: true',
    'tick-rates:',
    '  grass-spread: 400',
    '  container-update: 3',
  ].join('\n');

  const fileContents = new Map<string, string>([
    ['inst-1:/server.properties', serverProperties],
    ['inst-1:/paper-global.yml', paperGlobal],
    ['inst-1:/eula.txt', 'eula=true\n'],
    ['inst-1:/logs/latest.log', '[10:00:01] [ServerMain/INFO]: Starting minecraft server version 1.21.4\n[10:00:12] [Server thread/INFO]: Done (11.2s)! For help, type "help"\n'],
  ]);

  const configs: ConfigDocument[] = [
    { id: 'cfg-1', instanceId: 'inst-1', path: '/server.properties', format: 'PROPERTIES', recognized: true, revision: 3, updatedAt: now - 2 * DAY, schemaTitle: 'Minecraft 服务器主配置' },
    { id: 'cfg-2', instanceId: 'inst-1', path: '/paper-global.yml', format: 'YAML', recognized: true, revision: 1, updatedAt: now - 2 * DAY, schemaTitle: 'Paper 全局配置' },
    { id: 'cfg-3', instanceId: 'inst-1', path: '/plugins/LuckPerms/config.yml', format: 'YAML', recognized: false, revision: 0, updatedAt: now - 20 * DAY, schemaTitle: null },
  ];

  const configFields = new Map<string, ConfigField[]>([
    [
      'cfg-1',
      [
        { key: 'motd', title: '服务器标语 (MOTD)', description: '显示在服务器列表中的介绍文字。', type: 'string', value: 'MCNP 生存主服', defaultValue: 'A Minecraft Server' },
        { key: 'server-port', title: '监听端口', description: '服务器监听的 TCP 端口。', type: 'number', value: 25565, defaultValue: 25565 },
        { key: 'max-players', title: '最大玩家数', description: null, type: 'number', value: 50, defaultValue: 20 },
        { key: 'online-mode', title: '正版验证', description: '关闭后允许离线账号进入（存在安全风险）。', type: 'boolean', value: true, defaultValue: true },
        { key: 'difficulty', title: '难度', description: null, type: 'enum', enumValues: ['peaceful', 'easy', 'normal', 'hard'], value: 'normal', defaultValue: 'easy' },
        { key: 'view-distance', title: '视距', description: '影响服务器性能的关键参数。', type: 'number', value: 10, defaultValue: 10 },
        { key: 'simulation-distance', title: '模拟距离', description: null, type: 'number', value: 8, defaultValue: 10 },
        { key: 'pvp', title: '允许 PVP', description: null, type: 'boolean', value: true, defaultValue: true },
        { key: 'white-list', title: '白名单', description: null, type: 'boolean', value: false, defaultValue: false },
        { key: 'spawn-protection', title: '出生点保护半径', description: null, type: 'number', value: 16, defaultValue: 16 },
      ],
    ],
    [
      'cfg-2',
      [
        { key: 'chunk-loading.enable-frustum-priority', title: '视锥优先加载', description: '优先加载玩家视野内的区块。', type: 'boolean', value: false, defaultValue: false },
        { key: 'misc.lag-compensate-block-breaking', title: '挖掘延迟补偿', description: null, type: 'boolean', value: true, defaultValue: true },
        { key: 'tick-rates.grass-spread', title: '草方块蔓延速率', description: '值越大蔓延越慢。', type: 'number', value: 400, defaultValue: 400 },
      ],
    ],
  ]);

  const configRaw = new Map<string, string>([
    ['cfg-1', serverProperties],
    ['cfg-2', paperGlobal],
    ['cfg-3', 'storage-method: h2\nsync-minutes: 3\n# LuckPerms 主配置（未识别 Schema，使用原始文本编辑）\n'],
  ]);

  const extensions: ExtensionInstall[] = [
    { id: 'ext-1', instanceId: 'inst-1', source: 'HANGAR', projectId: 'LuckPerms', name: 'LuckPerms', version: '5.4.141', sha256: '9f8e7d', fileName: 'LuckPerms-Bukkit-5.4.141.jar', installedAt: now - 20 * DAY, updateAvailable: null },
    { id: 'ext-2', instanceId: 'inst-1', source: 'MODRINTH', projectId: 'chunky', name: 'Chunky', version: '1.4.28', sha256: '1a2b3c', fileName: 'Chunky-Bukkit-1.4.28.jar', installedAt: now - 20 * DAY, updateAvailable: '1.4.36' },
    { id: 'ext-3', instanceId: 'inst-1', source: 'MODRINTH', projectId: 'spark', name: 'spark', version: '1.10.109', sha256: '4d5e6f', fileName: 'spark-1.10.109-bukkit.jar', installedAt: now - 20 * DAY, updateAvailable: null },
  ];

  const extensionCatalog: ExtensionProject[] = [
    { source: 'MODRINTH', projectId: 'chunky', name: 'Chunky', summary: '区块预生成工具，可控制生成速率与形状。', downloads: 4_200_000, updatedAt: now - 5 * DAY, url: 'https://modrinth.com/plugin/chunky' },
    { source: 'MODRINTH', projectId: 'spark', name: 'spark', summary: '性能分析器：CPU、内存、Tick 报告。', downloads: 12_800_000, updatedAt: now - 12 * DAY, url: 'https://modrinth.com/plugin/spark' },
    { source: 'HANGAR', projectId: 'LuckPerms', name: 'LuckPerms', summary: '权限管理插件，支持上下文与临时权限。', downloads: 38_000_000, updatedAt: now - 30 * DAY, url: 'https://hangar.papermc.io/Luck/LuckPerms' },
    { source: 'MODRINTH', projectId: 'viaversion', name: 'ViaVersion', summary: '允许新版本客户端连接旧版本服务端。', downloads: 25_000_000, updatedAt: now - 8 * DAY, url: 'https://modrinth.com/plugin/viaversion' },
    { source: 'MODRINTH', projectId: 'coreprotect', name: 'CoreProtect', summary: '方块记录与回滚，查熊必备。', downloads: 9_600_000, updatedAt: now - 15 * DAY, url: 'https://modrinth.com/plugin/coreprotect' },
    { source: 'MODRINTH', projectId: 'squaremap', name: 'squaremap', summary: '轻量网页地图插件。', downloads: 1_100_000, updatedAt: now - 3 * DAY, url: 'https://modrinth.com/plugin/squaremap' },
  ];

  const images: ImageInfo[] = [
    { id: 'img-1', coreId: 'core-1', repoTags: ['itzg/minecraft-server:latest'], sizeBytes: 712_000_000, createdAt: now - 15 * DAY },
    { id: 'img-2', coreId: 'core-1', repoTags: ['eclipse-temurin:21-jre'], sizeBytes: 246_000_000, createdAt: now - 15 * DAY },
  ];

  const builds: ImageBuild[] = [
    { id: 'build-1', coreId: 'core-1', tag: 'mcnp/custom-paper:1.21.4', status: 'SUCCESS', startedAt: now - 10 * DAY, finishedAt: now - 10 * DAY + 240_000 },
  ];

  const cpuTopologies: CpuTopology[] = [
    {
      coreId: 'core-1',
      logicalCpus: 16,
      performanceCores: [0, 1, 2, 3, 4, 5, 6, 7],
      efficiencyCores: [8, 9, 10, 11, 12, 13, 14, 15],
      numaNodes: [{ id: 0, cpus: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15] }],
    },
    {
      coreId: 'core-2',
      logicalCpus: 12,
      performanceCores: [0, 1, 2, 3, 4, 5],
      efficiencyCores: [6, 7, 8, 9, 10, 11],
      numaNodes: [{ id: 0, cpus: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11] }],
    },
  ];

  const cpuPolicies = new Map<string, CpuPolicy>([
    ['inst-1', { instanceId: 'inst-1', mode: 'AUTO_PERFORMANCE', cpuSet: [0, 1, 2, 3, 4, 5, 6, 7], exclusive: false, numaNodeId: null, status: 'applied' }],
    ['inst-2', { instanceId: 'inst-2', mode: 'SHARED', cpuSet: [], exclusive: false, numaNodeId: null, status: 'applied' }],
    ['inst-3', { instanceId: 'inst-3', mode: 'MANUAL', cpuSet: [8, 9], exclusive: false, numaNodeId: null, status: 'applied' }],
    ['inst-4', { instanceId: 'inst-4', mode: 'AUTO_PERFORMANCE', cpuSet: [0, 1, 2, 3, 4, 5], exclusive: true, numaNodeId: 0, status: 'degraded' }],
    ['inst-5', { instanceId: 'inst-5', mode: 'SHARED', cpuSet: [], exclusive: false, numaNodeId: null, status: 'requested' }],
  ]);

  const schedules: Schedule[] = [
    {
      id: 'sch-1',
      instanceId: 'inst-1',
      name: '每日凌晨重启',
      enabled: true,
      trigger: { kind: 'CRON', cron: '0 5 * * *', timezone: 'Asia/Shanghai' },
      action: 'restart',
      lastRunAt: now - 21 * HOUR,
      nextRunAt: now + 3 * HOUR,
    },
    {
      id: 'sch-2',
      instanceId: 'inst-1',
      name: '每 6 小时备份',
      enabled: true,
      trigger: { kind: 'CRON', cron: '15 */6 * * *', timezone: 'Asia/Shanghai' },
      action: 'backup',
      lastRunAt: now - 4 * HOUR,
      nextRunAt: now + 2 * HOUR,
    },
    {
      id: 'sch-3',
      instanceId: 'inst-1',
      name: '崩溃后广播告警',
      enabled: false,
      trigger: { kind: 'EVENT', event: 'EXIT_CODE_NONZERO' },
      action: 'command:say 服务器异常退出，管理员已收到通知',
      lastRunAt: null,
      nextRunAt: null,
    },
  ];

  const executions: ScheduleExecution[] = [
    { id: 'exec-1', scheduleId: 'sch-1', startedAt: now - 21 * HOUR, finishedAt: now - 21 * HOUR + 95_000, result: 'SUCCESS', message: '实例已重启' },
    { id: 'exec-2', scheduleId: 'sch-2', startedAt: now - 4 * HOUR, finishedAt: now - 4 * HOUR + 300_000, result: 'SUCCESS', message: '备份完成：backup-2024-12-10-04.tar.zst (1.2 GiB)' },
    { id: 'exec-3', scheduleId: 'sch-2', startedAt: now - 10 * HOUR, finishedAt: now - 10 * HOUR + 280_000, result: 'SUCCESS', message: '备份完成：backup-2024-12-09-22.tar.zst (1.2 GiB)' },
    { id: 'exec-4', scheduleId: 'sch-1', startedAt: now - 45 * HOUR, finishedAt: now - 45 * HOUR + 12_000, result: 'FAILED', message: '停止超时，已强制终止' },
  ];

  const groups: Group[] = [
    {
      id: 'g-admin',
      name: '管理员',
      description: '内置组：拥有全部权限，范围不受限。',
      permissions: [...ALL_PERMISSIONS],
      instanceScopes: { coreIds: [], instanceIds: [], tagSelectors: [] },
      builtIn: true,
    },
    {
      id: 'g-ops',
      name: '运维组',
      description: '日常运维：实例控制、终端、文件、配置、扩展与计划任务，限上海节点。',
      permissions: [
        'core.read',
        'environment.read',
        'environment.manage',
        'instance.read',
        'instance.control',
        'instance.console.read',
        'instance.console.write',
        'instance.settings.basic',
        'instance.settings.launch',
        'file.read',
        'file.write',
        'config.read',
        'config.write',
        'extension.read',
        'extension.manage',
        'schedule.read',
        'schedule.manage',
      ],
      instanceScopes: { coreIds: ['core-1'], instanceIds: [], tagSelectors: [] },
      builtIn: false,
    },
    {
      id: 'g-view',
      name: '只读观察',
      description: '仅可查看实例与终端，限创造测试服。',
      permissions: ['core.read', 'instance.read', 'instance.console.read', 'config.read', 'file.read', 'schedule.read'],
      instanceScopes: { coreIds: [], instanceIds: ['inst-2'], tagSelectors: [] },
      builtIn: false,
    },
  ];

  const users: User[] = [
    { id: 'u-admin', username: 'admin', displayName: '站长', groupIds: ['g-admin'], disabled: false, lastLoginAt: now - 2 * HOUR, createdAt: now - 90 * DAY },
    { id: 'u-op', username: 'operator', displayName: '运维小王', groupIds: ['g-ops'], disabled: false, lastLoginAt: now - 26 * HOUR, createdAt: now - 60 * DAY },
    { id: 'u-view', username: 'viewer', displayName: '观察者', groupIds: ['g-view'], disabled: false, lastLoginAt: now - 5 * DAY, createdAt: now - 30 * DAY },
  ];

  const sessions: DeviceSession[] = [
    { id: 'sess-1', userId: 'u-admin', deviceName: 'Chrome / Windows', platform: 'browser', ip: '203.0.113.10', createdAt: now - 2 * HOUR, lastActiveAt: now - 60_000, current: true },
    { id: 'sess-2', userId: 'u-admin', deviceName: 'MCNP Desktop / macOS', platform: 'tauri-desktop', ip: '203.0.113.10', createdAt: now - 6 * DAY, lastActiveAt: now - 2 * DAY, current: false },
  ];

  const tasks: Task[] = [
    { id: 'task-1', kind: 'RUNTIME_INSTALL', title: '安装 Java 21.0.5+11（备用节点-家用机）', status: 'SUCCESS', progress: 1, message: '校验通过 (SHA-256)', coreId: 'core-2', instanceId: null, createdAt: now - 25 * DAY, finishedAt: now - 25 * DAY + 90_000, cancellable: false },
    { id: 'task-2', kind: 'BACKUP', title: '备份 生存主服', status: 'SUCCESS', progress: 1, message: 'backup-2024-12-10-04.tar.zst (1.2 GiB)', coreId: 'core-1', instanceId: 'inst-1', createdAt: now - 4 * HOUR, finishedAt: now - 4 * HOUR + 300_000, cancellable: false },
    { id: 'task-3', kind: 'INSTANCE_CREATE', title: '创建实例 周末活动服', status: 'SUCCESS', progress: 1, message: null, coreId: 'core-2', instanceId: 'inst-5', createdAt: now - 2 * DAY, finishedAt: now - 2 * DAY + 12_000, cancellable: false },
  ];

  const audit: AuditEvent[] = [
    { id: 'aud-1', actorId: 'u-admin', actorName: 'admin', action: 'instance.start', target: 'inst-1 (生存主服)', result: 'SUCCESS', requestId: 'req-8f31c2', sourceIp: '203.0.113.10', createdAt: now - 26 * HOUR, detail: null },
    { id: 'aud-2', actorId: 'u-op', actorName: 'operator', action: 'config.write', target: 'inst-1:/server.properties', result: 'SUCCESS', requestId: 'req-8f2aa0', sourceIp: '198.51.100.7', createdAt: now - 2 * DAY, detail: 'revision 2 → 3，字段 motd、view-distance' },
    { id: 'aud-3', actorId: 'u-view', actorName: 'viewer', action: 'file.write', target: 'inst-2:/server.properties', result: 'DENIED', requestId: 'req-8e77d1', sourceIp: '192.0.2.44', createdAt: now - 5 * DAY, detail: '缺少权限 file.write' },
    { id: 'aud-4', actorId: 'u-admin', actorName: 'admin', action: 'core.create', target: 'core-2 (备用节点-家用机)', result: 'SUCCESS', requestId: 'req-77aa10', sourceIp: '203.0.113.10', createdAt: now - 30 * DAY, detail: null },
    { id: 'aud-5', actorId: 'u-admin', actorName: 'admin', action: 'user.create', target: 'viewer', result: 'SUCCESS', requestId: 'req-6601bc', sourceIp: '203.0.113.10', createdAt: now - 30 * DAY, detail: '用户组：只读观察' },
    { id: 'aud-6', actorId: 'u-op', actorName: 'operator', action: 'instance.control', target: 'inst-4 (模组 Fabric 服)', result: 'FAILED', requestId: 'req-64d219', sourceIp: '198.51.100.7', createdAt: now - 3 * DAY, detail: '实例退出码 1，见控制台日志' },
    { id: 'aud-7', actorId: 'u-admin', actorName: 'admin', action: 'auth.login', target: 'admin', result: 'SUCCESS', requestId: 'req-5c20ef', sourceIp: '203.0.113.10', createdAt: now - 2 * HOUR, detail: 'Chrome / Windows' },
    { id: 'aud-8', actorId: 'u-op', actorName: 'operator', action: 'extension.manage', target: 'inst-1 (生存主服)', result: 'SUCCESS', requestId: 'req-51b9a4', sourceIp: '198.51.100.7', createdAt: now - 20 * DAY, detail: '安装 Chunky 1.4.28 (MODRINTH)' },
  ];

  return {
    users,
    groups,
    sessions,
    credentials: { admin: 'admin123', operator: 'operator123', viewer: 'viewer123' },
    cores,
    instances,
    runtimes,
    managedRuntimes,
    templates,
    fileTree,
    fileContents,
    configs,
    configFields,
    configRaw,
    extensions,
    extensionCatalog,
    images,
    builds,
    cpuTopologies,
    cpuPolicies,
    schedules,
    executions,
    tasks,
    audit,
  };
}
