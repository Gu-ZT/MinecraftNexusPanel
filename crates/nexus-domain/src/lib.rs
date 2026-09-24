//! MCNP 纯领域类型层。
//!
//! 承载 PLAN.md 第 6 节列出的全部领域对象：CoreNode、Instance、InstanceRuntime、
//! ManagedRuntime、InstallTemplate/InstallSource、ConfigDocument/ConfigSchema、
//! ExtensionProject/ExtensionInstall、Image/ImageBuild、CpuTopology/CpuPolicy/CpuReservation、
//! Schedule/Trigger/Execution、Task、User/Group/Role/Permission/ResourceScope、
//! Session/Device、AuditEvent，以及实例状态机（CREATED→STARTING→RUNNING→STOPPING→STOPPED/FAILED）。
//!
//! 本 crate 不依赖任何 IO / 框架，供 protocol、core、panel、storage 共同引用。
//!
//! TODO(M0): 冻结 v1 领域类型与实例状态机定义。
