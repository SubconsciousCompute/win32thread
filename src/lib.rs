use crossbeam_channel::Sender;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
use wmi::{COMLibrary, FilterValue, WMIConnection, WMIDateTime};

// see https://docs.rs/wmi/latest/wmi/#subscribing-to-event-notifications for further explanation

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceCreationEvent")]
#[serde(rename_all = "PascalCase")]
struct NewThreadEvent {
    target_instance: Win32_Thread,
}

/// The Win32_Process WMI class represents a process on an operating system.
/// https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-process
#[derive(Deserialize, Debug)]
pub struct Win32_Thread {
    Caption: Option<String>,
    CreationClassName: Option<String>,
    CSCreationClassName: Option<String>,
    CSName: Option<String>,
    Description: Option<String>,
    ElapsedTime: Option<u64>,
    ExecutionState: Option<u16>,
    Handle: Option<String>,
    InstallDate: Option<WMIDateTime>,
    KernelModeTime: Option<u64>,
    Name: Option<String>,
    OSCreationClassName: Option<String>,
    OSName: Option<String>,
    Priority: Option<u32>,
    PriorityBase: Option<u32>,
    ProcessCreationClassName: Option<String>,
    ProcessHandle: Option<String>,
    StartAddress: Option<u32>,
    Status: Option<String>,
    ThreadState: Option<u32>,
    ThreadWaitReason: Option<u32>,
    UserModeTime: Option<u64>,
}

/// Thread space sensor.
pub struct ProcessThread {
    tx: Option<Sender<Win32_Thread>>,
}

impl ProcessThread {
    pub fn new(tx: Option<Sender<Win32_Thread>>) -> Self {
        Self { tx }
    }

    fn connect(&mut self) -> anyhow::Result<WMIConnection> {
        let com_con = COMLibrary::new()?;
        let wmi_con = WMIConnection::new(com_con)?;
        tracing::info!("WMI connection created.");
        Ok(wmi_con)
    }

    /// Collect the information in a vector. If `self.tx` is `Some`, then we send the information
    /// over the channel and returns empty vector.
    pub fn collect(&mut self) -> anyhow::Result<Vec<Win32_Thread>> {
        let wmi_con = self.connect().unwrap();

        let mut filters = HashMap::<String, FilterValue>::new();
        filters.insert(
            "TargetInstance".to_owned(),
            FilterValue::is_a::<Win32_Thread>()?,
        );

        let mut ps = vec![];
        for result in wmi_con
            .filtered_notification::<NewThreadEvent>(&filters, Some(Duration::from_secs(1)))?
        {
            let mut thread = result?.target_instance;
            if let Some(tx) = &self.tx {
                if let Err(e) = tx.send(thread) {
                    tracing::error!("Failed to send process. Error {e:#?}.");
                }
            } else {
                ps.push(thread);
            }
        }
        Ok(ps)
    }

    /// Run indefinitely and send information over channel.
    pub fn run(&mut self) -> anyhow::Result<()> {
        // Before using WMI, a connection must be created.
        let com_con = COMLibrary::new().unwrap();
        let wmi_con = WMIConnection::new(com_con).unwrap();
        let mut filters = HashMap::<String, FilterValue>::new();
        filters.insert(
            "TargetInstance".to_owned(),
            FilterValue::is_a::<Win32_Thread>()?,
        );
        tracing::info!("WMI connection created.");
        loop {
            for result in wmi_con
                .filtered_notification::<NewThreadEvent>(&filters, Some(Duration::from_secs(1)))?
            {
                let mut thread = result?.target_instance;
                if let Some(tx) = &self.tx {
                    if let Err(e) = tx.send(thread) {
                        eprintln!("Error sending {e:?}");
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
