//! Microsoft → Xbox Live → XSTS → Minecraft authentication (Phase 2).

/// Azure application (client) ID. Public by design: native apps cannot keep secrets.
/// Requires Mojang allow-listing (https://aka.ms/mce-reviewappid) to reach api.minecraftservices.com.
pub const MSA_CLIENT_ID: &str = "47ec2f94-9a22-4089-95c2-e2cbbc4afd44";
