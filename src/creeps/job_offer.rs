use super::Job;

pub struct JobOffer {
    pub available_places: u32,
    pub job: Job,
}

impl JobOffer {
    pub fn new(job: Job, available_places: u32) -> Self {
        Self {
            job,
            available_places,
        }
    }
}
