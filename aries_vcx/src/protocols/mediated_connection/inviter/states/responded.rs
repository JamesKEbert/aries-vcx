use std::clone::Clone;

use messages::diddoc::aries::diddoc::AriesDidDoc;

use messages::protocols::connection::problem_report::ProblemReport;
use messages::protocols::connection::response::SignedResponse;

use crate::protocols::mediated_connection::inviter::states::completed::CompletedState;
use crate::protocols::mediated_connection::inviter::states::initial::InitialState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RespondedState {
    pub signed_response: SignedResponse,
    pub did_doc: AriesDidDoc,
}

impl From<(RespondedState, ProblemReport)> for InitialState {
    fn from((_state, problem_report): (RespondedState, ProblemReport)) -> InitialState {
        trace!(
            "ConnectionInviter: transit state from RespondedState to InitialState, problem_report: {:?}",
            problem_report
        );
        InitialState::new(Some(problem_report))
    }
}

impl From<RespondedState> for CompletedState {
    fn from(state: RespondedState) -> CompletedState {
        trace!("ConnectionInviter: transit state from RespondedState to CompleteState");
        CompletedState {
            did_doc: state.did_doc,
            thread_id: Some(state.signed_response.get_thread_id()),
            protocols: None,
        }
    }
}