use super::TargetedRunnerTaskMsg;
use crate::hash_types::worker;
use crate::worker::Error;
use crate::{proto::SimulationShortID, types::TaskID, Language};
use nng::Message;
use std::convert::{TryFrom, TryInto};

#[derive(Debug, Default, Clone)]
pub struct RunnerError {
    pub message: Option<String>,
    pub details: Option<String>,
    pub file_name: Option<String>,
    pub line_number: Option<i32>,
}

impl RunnerError {
    pub fn into_sendable(self, is_warning: bool) -> worker::RunnerError {
        worker::RunnerError {
            message: self.message,
            code: None,
            line_number: self.line_number,
            file_name: self.file_name,
            details: self.details,
            is_warning,
            is_internal: false,
        }
    }
}

impl From<crate::gen::runner_error_generated::RunnerError<'_>> for RunnerError {
    fn from(runner_error: crate::gen::runner_error_generated::RunnerError) -> Self {
        Self {
            message: runner_error.msg().map(|msg| msg.to_string()),
            // TODO: these are currently not encapsulated within the Flatbuffers
            details: None,
            file_name: None,
            line_number: None,
        }
    }
}

impl From<crate::gen::runner_warning_generated::RunnerWarning<'_>> for RunnerError {
    fn from(runner_warning: crate::gen::runner_warning_generated::RunnerWarning) -> Self {
        Self {
            message: Some(runner_warning.msg().to_string()),
            details: runner_warning.details().map(|details| details.to_string()),
            // TODO: these are currently not encapsulated within the Flatbuffers
            file_name: None,
            line_number: None,
        }
    }
}

#[derive(Debug)]
pub enum OutboundFromRunnerMsgPayload {
    TaskMsg(TargetedRunnerTaskMsg),
    TaskCancelled(TaskID),
    RunnerError(RunnerError),
    RunnerErrors(Vec<RunnerError>),
    RunnerWarning(RunnerError),
    RunnerWarnings(Vec<RunnerError>),
    // TODO: add
    // PackageError
    // UserErrors
    // UserWarnings
}

impl TryFrom<crate::gen::runner_outbound_msg_generated::RunnerOutboundMsg<'_>>
    for OutboundFromRunnerMsgPayload
{
    type Error = Error;

    fn try_from(
        parsed_msg: crate::gen::runner_outbound_msg_generated::RunnerOutboundMsg,
    ) -> Result<Self, Self::Error> {
        Ok(match parsed_msg.payload_type() {
            crate::gen::runner_outbound_msg_generated::RunnerOutboundMsgPayload::NONE => {
                return Err(Error::from("Message from runner had no payload"))
            }
            crate::gen::runner_outbound_msg_generated::RunnerOutboundMsgPayload::TaskMsg => {
                let payload = parsed_msg.payload_as_task_msg().ok_or_else(|| {
                    Error::from(
                        "Message from runner should have had a TaskMsg payload but it was missing",
                    )
                })?;
                Self::TaskMsg(payload.try_into()?)
            }
            crate::gen::runner_outbound_msg_generated::RunnerOutboundMsgPayload::TaskCancelled => {
                let payload = parsed_msg.payload_as_task_cancelled().ok_or_else(|| Error::from(
                        "Message from runner should have had a TaskCancelled payload but it was missing",
                    ))?;

                let task_id = payload.task_id().ok_or_else(|| {
                    Error::from("Message from runner should have had a task_id but it was missing")
                })?;

                Self::TaskCancelled(uuid::Uuid::from_slice(&task_id.0)?.as_u128())
            }
            crate::gen::runner_outbound_msg_generated::RunnerOutboundMsgPayload::RunnerError => {
                let payload = parsed_msg.payload_as_runner_error().ok_or_else(|| {
                    Error::from(
                    "Message from runner should have had a RunnerError payload but it was missing",
                )
                })?;

                Self::RunnerError(payload.into())
            }
            crate::gen::runner_outbound_msg_generated::RunnerOutboundMsgPayload::RunnerErrors => {
                let payload = parsed_msg.payload_as_runner_errors().ok_or_else(|| {
                    Error::from(
                    "Message from runner should have had a RunnerErrors payload but it was missing",
                )
                })?;
                let runner_errors = payload
                    .inner()
                    .iter()
                    .map(|runner_error| runner_error.into())
                    .collect();
                Self::RunnerErrors(runner_errors)
            }
            crate::gen::runner_outbound_msg_generated::RunnerOutboundMsgPayload::RunnerWarning => {
                let payload = parsed_msg.payload_as_runner_warning().ok_or_else(|| Error::from(
                        "Message from runner should have had a RunnerWarning payload but it was missing",
                    ))?;

                Self::RunnerWarning(payload.into())
            }
            crate::gen::runner_outbound_msg_generated::RunnerOutboundMsgPayload::RunnerWarnings => {
                let payload = parsed_msg.payload_as_runner_warnings().ok_or_else(|| Error::from(
                        "Message from runner should have had a RunnerWarnings payload but it was missing",
                    ))?;

                let runner_warnings = payload
                    .inner()
                    .iter()
                    .map(|runner_warning| runner_warning.into())
                    .collect();
                Self::RunnerWarnings(runner_warnings)
            }
            crate::gen::runner_outbound_msg_generated::RunnerOutboundMsgPayload::PackageError => {
                let payload = parsed_msg.payload_as_package_error().ok_or_else(|| {
                    Error::from(
                    "Message from runner should have had a PackageError payload but it was missing",
                )
                })?;

                todo!() // TODO: there is no Self::PackageError
            }
            crate::gen::runner_outbound_msg_generated::RunnerOutboundMsgPayload::UserErrors => {
                let payload = parsed_msg.payload_as_user_errors().ok_or_else(|| {
                    Error::from(
                    "Message from runner should have had a UserErrors payload but it was missing",
                )
                })?;

                todo!() // TODO: there is no Self::UserErrors
            }
            crate::gen::runner_outbound_msg_generated::RunnerOutboundMsgPayload::UserWarnings => {
                let payload = parsed_msg.payload_as_user_warnings().ok_or_else(|| {
                    Error::from(
                    "Message from runner should have had a UserWarnings payload but it was missing",
                )
                })?;

                todo!() // TODO: there is no Self::UserWarnings
            }
        })
    }
}

#[derive(Debug)]
pub struct OutboundFromRunnerMsg {
    pub source: Language,
    pub sim_id: SimulationShortID,
    pub payload: OutboundFromRunnerMsgPayload,
    // shared state
}

impl TryFrom<nng::Message> for OutboundFromRunnerMsg {
    type Error = Error;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        let bytes = message.as_slice();
        let parsed_msg = crate::gen::runner_outbound_msg_generated::root_as_runner_outbound_msg(
            bytes,
        )
        .map_err(|err| {
            Error::from(format!(
                "Flatbuffers failed to parse message bytes as a RunnerOutboundMsg: {}",
                err.to_string()
            ))
        })?;
        Ok(Self {
            source: Language::Python,
            sim_id: parsed_msg.sim_sid(),
            payload: parsed_msg.try_into()?,
        })
    }
}
