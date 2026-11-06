macro_rules! delegate_utility_vm_runtime_driver {
    ($driver:ty, $inner:ident) => {
        #[koci_sdk::async_trait]
        impl $crate::RuntimeDriver for $driver {
            fn capability(&self) -> koci_core::DriverCapability {
                $crate::RuntimeDriver::capability(&self.$inner)
            }

            fn linux_support(&self) -> koci_sdk::Result<koci_sdk::OciLinuxSupport> {
                $crate::RuntimeDriver::linux_support(&self.$inner)
            }

            fn operations(&self) -> &[koci_sdk::RuntimeOperation] {
                $crate::RuntimeDriver::operations(&self.$inner)
            }

            fn hooks(&self) -> &[$crate::OciHookPhase] {
                $crate::RuntimeDriver::hooks(&self.$inner)
            }

            fn attachment_capabilities(&self) -> koci_sdk::AttachmentCapabilities {
                $crate::RuntimeDriver::attachment_capabilities(&self.$inner)
            }

            async fn acknowledge_operation(
                &self,
                operation_id: &koci_sdk::OperationId,
            ) -> koci_sdk::Result<()> {
                $crate::RuntimeDriver::acknowledge_operation(&self.$inner, operation_id).await
            }

            async fn prepare_create_bundle(
                &self,
                request: &$crate::DriverCreateRequest,
            ) -> koci_sdk::Result<koci_sdk::OciBundle> {
                $crate::RuntimeDriver::prepare_create_bundle(&self.$inner, request).await
            }

            async fn recover(
                &self,
                record: &koci_sdk::ContainerRecord,
            ) -> koci_sdk::Result<$crate::DriverRecovery> {
                $crate::RuntimeDriver::recover(&self.$inner, record).await
            }

            async fn create(
                &self,
                request: $crate::DriverCreateRequest,
            ) -> koci_sdk::Result<$crate::DriverState> {
                $crate::RuntimeDriver::create(&self.$inner, request).await
            }

            async fn state(
                &self,
                target: koci_sdk::ContainerTarget,
            ) -> koci_sdk::Result<$crate::DriverState> {
                $crate::RuntimeDriver::state(&self.$inner, target).await
            }

            async fn start(
                &self,
                request: $crate::DriverStartRequest,
            ) -> koci_sdk::Result<$crate::DriverState> {
                $crate::RuntimeDriver::start(&self.$inner, request).await
            }

            async fn kill(
                &self,
                request: $crate::DriverKillRequest,
            ) -> koci_sdk::Result<$crate::DriverState> {
                $crate::RuntimeDriver::kill(&self.$inner, request).await
            }

            async fn delete(&self, request: $crate::DriverDeleteRequest) -> koci_sdk::Result<()> {
                $crate::RuntimeDriver::delete(&self.$inner, request).await
            }

            async fn wait(
                &self,
                request: $crate::DriverWaitRequest,
            ) -> koci_sdk::Result<koci_sdk::ExitStatus> {
                $crate::RuntimeDriver::wait(&self.$inner, request).await
            }

            async fn exec(
                &self,
                request: $crate::DriverExecRequest,
            ) -> koci_sdk::Result<$crate::DriverProcess> {
                $crate::RuntimeDriver::exec(&self.$inner, request).await
            }

            async fn signal_process(
                &self,
                request: $crate::DriverSignalProcessRequest,
            ) -> koci_sdk::Result<()> {
                $crate::RuntimeDriver::signal_process(&self.$inner, request).await
            }

            async fn wait_process(
                &self,
                request: $crate::DriverWaitProcessRequest,
            ) -> koci_sdk::Result<koci_sdk::ExitStatus> {
                $crate::RuntimeDriver::wait_process(&self.$inner, request).await
            }

            async fn pause(
                &self,
                request: $crate::DriverContainerOperationRequest,
            ) -> koci_sdk::Result<$crate::DriverState> {
                $crate::RuntimeDriver::pause(&self.$inner, request).await
            }

            async fn resume(
                &self,
                request: $crate::DriverContainerOperationRequest,
            ) -> koci_sdk::Result<$crate::DriverState> {
                $crate::RuntimeDriver::resume(&self.$inner, request).await
            }

            async fn processes(
                &self,
                target: koci_sdk::ContainerTarget,
            ) -> koci_sdk::Result<Vec<koci_sdk::ProcessRecord>> {
                $crate::RuntimeDriver::processes(&self.$inner, target).await
            }

            async fn update(
                &self,
                request: $crate::DriverUpdateRequest,
            ) -> koci_sdk::Result<$crate::DriverState> {
                $crate::RuntimeDriver::update(&self.$inner, request).await
            }

            async fn stats(
                &self,
                target: koci_sdk::ContainerTarget,
            ) -> koci_sdk::Result<koci_sdk::ContainerStats> {
                $crate::RuntimeDriver::stats(&self.$inner, target).await
            }

            async fn read_output(
                &self,
                request: $crate::DriverReadOutputRequest,
            ) -> koci_sdk::Result<Vec<koci_sdk::OutputChunk>> {
                $crate::RuntimeDriver::read_output(&self.$inner, request).await
            }

            async fn write_stdin(
                &self,
                request: $crate::DriverWriteStdinRequest,
            ) -> koci_sdk::Result<()> {
                $crate::RuntimeDriver::write_stdin(&self.$inner, request).await
            }

            async fn close_stdin(
                &self,
                request: $crate::DriverCloseStdinRequest,
            ) -> koci_sdk::Result<()> {
                $crate::RuntimeDriver::close_stdin(&self.$inner, request).await
            }

            async fn resize(&self, request: $crate::DriverResizeRequest) -> koci_sdk::Result<()> {
                $crate::RuntimeDriver::resize(&self.$inner, request).await
            }

            async fn file(
                &self,
                request: koci_sdk::FileRequest,
            ) -> koci_sdk::Result<koci_sdk::FileResponse> {
                $crate::RuntimeDriver::file(&self.$inner, request).await
            }

            async fn filesystem(
                &self,
                request: koci_sdk::FilesystemRequest,
            ) -> koci_sdk::Result<koci_sdk::FilesystemResponse> {
                $crate::RuntimeDriver::filesystem(&self.$inner, request).await
            }
        }
    };
}

pub(crate) use delegate_utility_vm_runtime_driver;
