use super::Job;

pub struct JobOffer {
    pub job: Job,
    pub taken: bool,
}

impl JobOffer {
    pub fn new(job: Job) -> Self {
        Self { job, taken: false }
    }
}
