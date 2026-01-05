-- Insert seed user
INSERT INTO users (user_id, username, password_hash)
VALUES (
    'eb1af277-a35c-48db-89ed-5f9c591e9096',
    'admin',
    '$argon2id$v=19$m=15000,t=2,p=1$rPaIWuEqedhgF85RAoZCgg$yxsFXLKdHHFf4+fyDZzAsZifGzZWxqtgmZca8p0F+1Y'
)