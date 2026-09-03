-- Make the creator identity explicit and keep anonymous entry read-only.
UPDATE room_roles SET role_key = 'admin', display_name = 'Admin'
WHERE role_key = 'manager';

UPDATE room_tokens SET role_key = 'admin'
WHERE role_key = 'manager';

UPDATE rooms SET default_role_key = 'reader'
WHERE default_role_key = 'editor';
