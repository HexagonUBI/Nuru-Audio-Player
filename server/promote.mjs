import { one, run } from './src/db.mjs';

const email = String(process.argv[2] ?? '').trim().toLowerCase();
const role = String(process.argv[3] ?? 'moderator').trim().toLowerCase();

if (!email) {
  console.error('usage: node server/promote.mjs <email> [moderator|user]');
  process.exit(1);
}

if (!['moderator', 'user'].includes(role)) {
  console.error(`unknown role ${role}, use moderator or user`);
  process.exit(1);
}

const user = one('SELECT id, email, name, role FROM users WHERE email = :email', { email });

if (!user) {
  console.error(`no account with the email ${email}`);
  process.exit(1);
}

if (user.role === role) {
  console.log(`${user.email} is already ${role}`);
  process.exit(0);
}

run('UPDATE users SET role = :role WHERE id = :id', { role, id: user.id });
console.log(`${user.email} (${user.name}) is now ${role}, was ${user.role}`);
