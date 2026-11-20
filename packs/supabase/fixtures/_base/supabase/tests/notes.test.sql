begin;
create extension if not exists pgtap with schema extensions;
select plan(2);
select has_table('public', 'notes', 'public.notes exists');
select policies_are('public', 'notes', array['notes_owner']);
select * from finish();
rollback;
