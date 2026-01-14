-- Add migration script here
CREATE TABLE issue_delivery_queue (
    issue_id uuid NOT NULL REFERENCES issues(issue_id),
    subscriber_email TEXT NOT NULL,
    PRIMARY KEY (issue_id, subscriber_email)
);