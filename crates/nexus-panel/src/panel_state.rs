use crate::AuthService;
use crate::CoreRegistry;
use crate::VersionMetadataClient;
use crate::WebSocketTicketStore;
use crate::extension_source_client::ExtensionSourceClient;
use crate::extension_task_store::ExtensionTaskStore;

/// 注入所有 Panel 路由共享服务的只读状态句柄集合。
#[derive(Clone)]
pub struct PanelState {
    auth: AuthService,
    cores: CoreRegistry,
    extension_sources: ExtensionSourceClient,
    extension_tasks: ExtensionTaskStore,
    version_metadata: VersionMetadataClient,
    websocket_tickets: WebSocketTicketStore,
}

impl PanelState {
    /// 创建 Panel 路由状态。
    #[must_use]
    pub fn new(
        auth: AuthService,
        cores: CoreRegistry,
        extension_sources: ExtensionSourceClient,
        version_metadata: VersionMetadataClient,
    ) -> Self {
        Self {
            auth,
            cores,
            extension_sources,
            extension_tasks: ExtensionTaskStore::default(),
            version_metadata,
            websocket_tickets: WebSocketTicketStore::default(),
        }
    }

    /// 返回认证服务。
    #[must_use]
    pub const fn auth(&self) -> &AuthService {
        &self.auth
    }

    /// 返回 Core 注册服务。
    #[must_use]
    pub const fn cores(&self) -> &CoreRegistry {
        &self.cores
    }

    /// 返回扩展来源客户端。
    #[must_use]
    pub const fn extension_sources(&self) -> &ExtensionSourceClient {
        &self.extension_sources
    }

    /// 返回扩展异步任务存储。
    #[must_use]
    pub const fn extension_tasks(&self) -> &ExtensionTaskStore {
        &self.extension_tasks
    }

    /// 返回安装模板版本元数据客户端。
    #[must_use]
    pub const fn version_metadata(&self) -> &VersionMetadataClient {
        &self.version_metadata
    }

    /// 返回 WebSocket 票据存储。
    #[must_use]
    pub const fn websocket_tickets(&self) -> &WebSocketTicketStore {
        &self.websocket_tickets
    }
}
