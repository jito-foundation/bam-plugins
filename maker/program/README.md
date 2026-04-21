# BAM Maker Plugin Program

This repository contains the on-chain BAM Maker Plugin registry program and its accompanying CLI. A program's upgrade authority or assigned delegate can enroll a program in BAM's Maker Prioritization Plugin.

## Authorities

The program currently uses these authorities:

- `upgrade_authority`: the target program's upgrade authority from the upgradeable loader
- `delegate_authority`: stored per enrolled program in `ProgramConfig`, assigned by the `upgrade_authority` and has the same permissions.
- `override_authority`: stored globally in `Config`, circuit-breaker authority with permissions to bypass the usual upgrade/delegate authorities.
- `status_authority`: stored globally in `Config`, used by the BAM Node set to activate enrolled configs after applying to block-building logic.
- `admin_authority`: stored globally in `Config`, used only for updating the global authority fields.

Permissions:

- `enroll`: `upgrade_authority` or `override_authority`
- `update market config`: `upgrade_authority`, `delegate_authority`, or `override_authority`
- `update program signer`: `upgrade_authority`, `delegate_authority`, or `override_authority`
- `update program memcmp`: `upgrade_authority`, `delegate_authority`, or `override_authority`
- `assign delegate authority`: `upgrade_authority` or `override_authority`
- `unenroll`: `upgrade_authority`, `delegate_authority`, or `override_authority`
- `override-unenroll`: `override_authority` only
- `activate`: `status_authority` only
- `admin-change-authority`: `admin_authority` only

## Program Status

On-chain `ProgramStatus` values:

- `Enrolled`: the program config exists and is pending activation
- `Active`: the BAM node set has applied the config
- `Unenroll`: the program has been marked for unenrollment
- `OverrideUnenroll`: the override authority has unenrolled the program

## Program Config Layout

`ProgramConfig` stores:
- `program_id`
- `delegate_authority`
- 32 program-level signer slots
- program-level `seqno_instruction_data_offset`
- 32 market config slots
- `status`

Each `MarketConfig` stores only:
- `market_id` - Automatically derived from `program_id` and market index
- `writable_account`

## Usage

Standard user flow:

1. Enroll the target program.
2. Optionally assign a delegate authority.
3. Add or replace market config slots with `update`.
4. Set program-level signer or seqno memcmp fields when needed.
5. Wait for the BAM node set to activate the config.
6. Unenroll later if needed.
