use crate::protocol::database::mysql::packet::{MySQLPacketHeader, MySQLPacketPayload, MySQLHandshakePacket, MySQLHandshakeResponse41Packet, MySQLOKPacket, MySQLPacket};
use bytes::Bytes;
use crate::protocol::database::mysql::constant::{MySQLCommandPacketType, MySQLAuthenticationMethod, MySQLCapabilityFlag};
use crate::protocol::database::{DatabasePacket, PacketPayload, CommandPacketType};
use crate::handler::mysql::binary::{ComStmtResetHandler, ComStmtCloseHandler, ComStmtExecuteHandler, ComStmtPrepareHandler};
use crate::handler::mysql::text::ComQueryHandler;
use crate::session::SessionContext;

pub mod text;
pub mod binary;
pub mod explainplan;
pub mod rdbc;

pub trait CommandHandler<P> {
    fn handle(command_packet_header: Option<MySQLPacketHeader>, command_packet: Option<P>, session_ctx: &mut SessionContext) -> Option<Vec<Bytes>>;
}

pub struct CommandRootHandler {}
impl CommandHandler<MySQLPacketPayload> for CommandRootHandler {
    fn handle(command_packet_header: Option<MySQLPacketHeader>, command_packet: Option<MySQLPacketPayload>, session_ctx: &mut SessionContext) -> Option<Vec<Bytes>> {
        let command_packet_header = command_packet_header.unwrap();
        let command_packet = command_packet.unwrap();
        let command_packet_type = command_packet_header.get_command_packet_type();
        match MySQLCommandPacketType::value_of(command_packet_type) {
            MySQLCommandPacketType::ComQuery => {
                ComQueryHandler::handle(Some(command_packet_header), Some(command_packet), session_ctx)
            },
            MySQLCommandPacketType::ComStmtPrepare => {
                ComStmtPrepareHandler::handle(Some(command_packet_header), Some(command_packet), session_ctx)
            },
            MySQLCommandPacketType::ComStmtExecute => {
                ComStmtExecuteHandler::handle(Some(command_packet_header), Some(command_packet), session_ctx)
            },
            MySQLCommandPacketType::ComStmtClose => {
                ComStmtCloseHandler::handle(Some(command_packet_header), Some(command_packet), session_ctx)
            },
            MySQLCommandPacketType::ComStmtReset => {
                ComStmtResetHandler::handle(Some(command_packet_header), Some(command_packet), session_ctx)
            },
            MySQLCommandPacketType::ComQuit => {
                ComQuitHandler::handle(Some(command_packet_header), None, session_ctx)
            },
            MySQLCommandPacketType::ComPing => {
                ComPingHandler::handle(Some(command_packet_header), None, session_ctx)
            },
            _ => {
                None
            }
        }
    }
}

pub struct HandshakeHandler {}
impl CommandHandler<MySQLPacketPayload> for HandshakeHandler {
    fn handle(command_packet_header: Option<MySQLPacketHeader>, command_packet: Option<MySQLPacketPayload>, session_ctx: &mut SessionContext) -> Option<Vec<Bytes>> {
        let mut handshake_packet = MySQLHandshakePacket::new(100); // TODO how to gen thread id
        let mut handshake_payload = MySQLPacketPayload::new();
        let handshake_payload = DatabasePacket::encode(&mut handshake_packet, &mut handshake_payload);
        Some(vec![handshake_payload.get_payload()])
    }
}

pub struct AuthHandler {}
impl CommandHandler<MySQLPacketPayload> for AuthHandler {
    fn handle(command_packet_header: Option<MySQLPacketHeader>, payload: Option<MySQLPacketPayload>, session_ctx: &mut SessionContext) -> Option<Vec<Bytes>> {
        let command_packet_header = command_packet_header.unwrap();
        let mut handshake_response41_payload = payload.unwrap();
        let mut handshake_response41_packet = MySQLHandshakeResponse41Packet::new();
        let handshake_response41_packet = DatabasePacket::decode(&mut handshake_response41_packet, &command_packet_header, &mut handshake_response41_payload, session_ctx);


        // TODO Auth Discovery
        let exists = true;
        if !handshake_response41_packet.get_database().is_empty() && !exists {

        }

        if (0 != (handshake_response41_packet.get_capability_flags() & (MySQLCapabilityFlag::ClientPluginAuth as u32)))
            && MySQLAuthenticationMethod::SecurePasswordAuthentication.value().to_string().eq(handshake_response41_packet.get_auth_plugin_name().as_str()) {

        }

        let mut ok_packet = MySQLOKPacket::new(handshake_response41_packet.get_sequence_id() + 1, 0, 0);
        let mut ok_payload = MySQLPacketPayload::new();
        let ok_payload = DatabasePacket::encode(&mut ok_packet, &mut ok_payload);

        Some(vec![ok_payload.get_payload()])
    }
}

pub struct ComQuitHandler {}
impl CommandHandler<MySQLPacketPayload> for ComQuitHandler {
    fn handle(command_packet_header: Option<MySQLPacketHeader>, command_packet: Option<MySQLPacketPayload>, session_ctx: &mut SessionContext) -> Option<Vec<Bytes>> {
        let mut ok_packet = MySQLOKPacket::new(1, 0, 0);
        let mut ok_payload = MySQLPacketPayload::new();
        let ok_payload = DatabasePacket::encode(&mut ok_packet, &mut ok_payload);
        Some(vec![ok_payload.get_payload()])
    }
}

pub struct ComPingHandler {}
impl CommandHandler<MySQLPacketPayload> for ComPingHandler {
    fn handle(command_packet_header: Option<MySQLPacketHeader>, command_packet: Option<MySQLPacketPayload>, session_ctx: &mut SessionContext) -> Option<Vec<Bytes>> {
        let mut ok_packet = MySQLOKPacket::new(1, 0, 0);
        let mut ok_payload = MySQLPacketPayload::new();
        let ok_payload = DatabasePacket::encode(&mut ok_packet, &mut ok_payload);
        Some(vec![ok_payload.get_payload()])
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use crate::discovery::database::RouteRules;
    use std::io::Read;
    use crate::parser::sql::mysql::parser;
    use crate::parser::sql::rewrite::SQLReWrite;
    use std::collections::HashMap;
    use crate::parser::sql::SQLStatementContext;
    use crate::parser::sql::analyse::SQLAnalyse;
    use mysql::Conn;
    use mysql::prelude::Queryable;
    use sqlparser::parser::Parser;

    #[test]
    fn test_route() {
        let sql = "SELECT a, b, 123, myfunc(b) \
           FROM t_order \
           WHERE user_id = 1 and a > b AND b < 100 \
           ORDER BY a DESC, b";
        //let sql = "insert into test (a, b, c) values (1, 1, ?)";
        let mut sql_ast = parser(sql.to_string());
        let sql_stmt = sql_ast.pop().unwrap();

        let mut file = File::open("./etc/dbmesh.yaml").expect("Unable to open file");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Unable to read file");
        let rules: RouteRules = serde_yaml::from_str(&contents).unwrap();

        let mut sql_stmt_ctx = SQLStatementContext::Default;
        sql_stmt.analyse(&mut sql_stmt_ctx).unwrap();

        let mut rewrite_sql = String::new();
        let ctx: HashMap<String, String> = HashMap::new();
        sql_stmt.rewrite(&mut rewrite_sql, &ctx).unwrap();

        assert_eq!(sql.to_uppercase(), rewrite_sql.to_uppercase());
    }

}