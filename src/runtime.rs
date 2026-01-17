use crate::config::{ProfileConfig, TargetConfig};
use crate::probe::{ProbeError, ProbeErrorKind, ProbeResult, ProbeSample};
use crate::probe_engine::ProbeClient;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use std::net::IpAddr;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug)]
pub enum ControlMessage {
    UpdateTarget(Box<TargetConfig>),
    UpdateProfile(Box<ProfileConfig>),
    Pause(bool),
    Stop,
}

pub struct WorkerHandle {
    pub sender: Sender<ControlMessage>,
    pub join: Option<JoinHandle<()>>,
}

pub fn spawn_profile_worker(
    target: TargetConfig,
    profile: ProfileConfig,
    sample_tx: Sender<ProbeSample>,
) -> WorkerHandle {
    let (tx, rx) = crossbeam_channel::unbounded();
    let join = thread::spawn(move || run_worker(target, profile, rx, sample_tx));
    WorkerHandle {
        sender: tx,
        join: Some(join),
    }
}

fn run_worker(
    mut target: TargetConfig,
    mut profile: ProfileConfig,
    control_rx: Receiver<ControlMessage>,
    sample_tx: Sender<ProbeSample>,
) {
    let mut paused = false;
    let mut resolved_ip: Option<IpAddr> = None;
    let mut client = match ProbeClient::new() {
        Ok(client) => client,
        Err(err) => {
            let _ = sample_tx.send(error_sample(
                target.id,
                profile.id,
                ProbeErrorKind::IoError,
                format!("probe client init failed: {err}"),
            ));
            return;
        }
    };

    // Perform initial probe immediately (don't wait for interval)
    let sample = client.probe(&target, &profile, resolved_ip);
    if let Some(remote) = sample.remote {
        resolved_ip = Some(remote.ip());
    }
    let _ = sample_tx.send(sample);

    loop {
        if paused {
            match control_rx.recv() {
                Ok(ControlMessage::Pause(flag)) => paused = flag,
                Ok(ControlMessage::UpdateTarget(cfg)) => target = *cfg,
                Ok(ControlMessage::UpdateProfile(cfg)) => profile = *cfg,
                Ok(ControlMessage::Stop) | Err(_) => break,
            }
            continue;
        }

        match control_rx.recv_timeout(target.interval) {
            Ok(ControlMessage::Pause(flag)) => paused = flag,
            Ok(ControlMessage::UpdateTarget(cfg)) => target = *cfg,
            Ok(ControlMessage::UpdateProfile(cfg)) => profile = *cfg,
            Ok(ControlMessage::Stop) => break,
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {
                let sample = client.probe(&target, &profile, resolved_ip);
                if let Some(remote) = sample.remote {
                    resolved_ip = Some(remote.ip());
                }
                let _ = sample_tx.send(sample);
            }
        }
    }
}

fn error_sample(
    target_id: crate::config::TargetId,
    profile_id: crate::config::ProfileId,
    kind: ProbeErrorKind,
    message: String,
) -> ProbeSample {
    ProbeSample {
        ts: SystemTime::now(),
        target_id,
        profile_id,
        result: ProbeResult::Err(ProbeError { kind, message }),
        http_status: None,
        negotiated: crate::probe::NegotiatedProtocol {
            alpn: None,
            tls_version: None,
            cipher: None,
        },
        t_dns: None,
        t_connect: Duration::from_millis(0),
        t_tls: None,
        t_ttfb: Duration::from_millis(0),
        t_download: Duration::from_millis(0),
        t_total: Duration::from_millis(0),
        downloaded_bytes: 0,
        local: None,
        remote: None,
        tcp_info: None,
        ebpf: None,
    }
}
