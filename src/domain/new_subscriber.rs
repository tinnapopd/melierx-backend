use crate::domain::SubscriberEmail;
use crate::domain::SubscriberName;

// Public Structs
pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}
