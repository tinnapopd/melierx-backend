use crate::domain::SubscriberEmail;
use crate::domain::SubscriberName;

/// Data structure representing a new subscriber.
pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}
