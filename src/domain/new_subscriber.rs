use crate::domain::SubscriberEmail;
use crate::domain::SubscriberName;

// Public Types
pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}
