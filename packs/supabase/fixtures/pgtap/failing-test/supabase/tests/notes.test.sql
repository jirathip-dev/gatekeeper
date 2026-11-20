begin;
create extension if not exists pgtap with schema extensions;
select plan(1);
select has_table('public', 'missing');
select * from finish();
rollback;
