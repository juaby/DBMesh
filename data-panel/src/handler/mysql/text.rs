use crate::handler::mysql::CommandHandler;
use crate::protocol::database::mysql::packet::{MySQLPacketPayload, MySQLPacketHeader, MySQLFieldCountPacket, MySQLColumnDefinition41Packet, MySQLEOFPacket, MySQLOKPacket};
use crate::protocol::database::{DatabasePacket, PacketPayload};
use sqlparser::ast::Statement;
use sqlparser::ast::SetVariableValue::Ident;
use crate::parser;
use mysql::{Value, Conn};
use bytes::Bytes;
use mysql::prelude::Queryable;
use crate::protocol::database::mysql::packet::text::{MySQLTextResultSetRowPacket, MySQLComQueryPacket};

pub struct ComQueryHandler {}
impl CommandHandler<MySQLPacketPayload> for ComQueryHandler {
    fn handle(command_packet_header: Option<MySQLPacketHeader>, command_packet: Option<MySQLPacketPayload>) -> Option<Vec<Bytes>> {
        let command_packet_header = command_packet_header.unwrap();
        let command_packet_type = command_packet_header.get_command_packet_type();
        let mut command_payload = command_packet.unwrap();
        let mut query_packet = MySQLComQueryPacket::new(command_packet_type);
        let command_packet = DatabasePacket::decode(&mut query_packet, &command_packet_header, &mut command_payload);

        let mut payloads = Vec::new();
        let database_url = "mysql://root:root@localhost:8306/test";
        let mut conn = Conn::new(database_url).unwrap();
        let command_sql = command_packet.get_sql();
        let sql = String::from_utf8_lossy(command_sql.as_slice());
        let sql = sql.to_string();
        println!("SQL = {}", sql);
        let statement = if sql.starts_with("SET") {
            Statement::SetVariable {
                local: false,
                variable: "".to_string(),
                value: Ident("".to_string())
            }
        } else {
            let mut statement = parser::mysql::parser(sql.as_str());
            let statement = statement.pop().unwrap();
            statement
        };

        match statement {
            Statement::Query(q) => {
                // This query will emit two result sets.
                let mut result = conn.query_iter((*q).to_string()).unwrap();

                let mut global_sequence_id: u32 = 1;

                while let Some(result_set) = result.next_set() {
                    let result_set = result_set.unwrap();

                    let columns = result_set.columns();
                    let columns_ref = columns.as_ref();
                    let columns_size = columns_ref.len();
                    let mut field_count_packet = MySQLFieldCountPacket::new(global_sequence_id, columns_size as u32);
                    let mut field_count_payload = MySQLPacketPayload::new();
                    let field_count_payload = DatabasePacket::encode(&mut field_count_packet, &mut field_count_payload);

                    payloads.push(field_count_payload.get_payload());

                    for c in columns_ref {
                        global_sequence_id = global_sequence_id + 1;
                        let sequence_id = global_sequence_id;
                        let character_set: u16 = c.character_set();
                        let flags: u16 = c.flags().bits() as u16;
                        let schema: String = c.schema_str().to_string();
                        let table: String = c.table_str().to_string();
                        let org_table: String = c.org_table_str().to_string();
                        let name: String = c.name_str().to_string();
                        let org_name: String = c.org_name_str().to_string();
                        let column_length: u32 = c.column_length();
                        let column_type: u8 = c.column_type() as u8; // MySQLColumnType
                        let decimals: u8 = c.decimals();
                        let mut column_definition41_packet =
                            MySQLColumnDefinition41Packet::new(
                                sequence_id,
                                character_set,
                                flags,
                                schema,
                                table,
                                org_table,
                                name,
                                org_name,
                                column_length,
                                column_type, // MySQLColumnType
                                decimals
                            );
                        let mut column_definition41_payload = MySQLPacketPayload::new();
                        let column_definition41_payload = DatabasePacket::encode(&mut column_definition41_packet, &mut column_definition41_payload);

                        payloads.push(column_definition41_payload.get_payload());
                    }

                    global_sequence_id = global_sequence_id + 1;
                    let mut eof_packet = MySQLEOFPacket::new(global_sequence_id);
                    let mut eof_payload = MySQLPacketPayload::new();
                    let eof_payload = DatabasePacket::encode(&mut eof_packet, &mut eof_payload);

                    payloads.push(eof_payload.get_payload());

                    for row in result_set {
                        let row = row.unwrap();
                        let mut datas: Vec<(bool, Vec<u8>)> = Vec::new();
                        for column_index in 0..columns_size {
                            let v = row.as_ref(column_index).unwrap();
                            let data = match v {
                                Value::Bytes(data) => (true, data.clone()),
                                Value::NULL => (false, Vec::new()),
                                _ => (true, Vec::new()),
                            };
                            datas.push(data);
                        }

                        global_sequence_id = global_sequence_id + 1;
                        let mut text_result_set_row_packet = MySQLTextResultSetRowPacket::new(global_sequence_id, datas);
                        let mut text_result_set_row_payload = MySQLPacketPayload::new();
                        let text_result_set_row_payload = DatabasePacket::encode(&mut text_result_set_row_packet, &mut text_result_set_row_payload);

                        payloads.push(text_result_set_row_payload.get_payload());
                    }

                    global_sequence_id = global_sequence_id + 1;
                    let mut eof_packet = MySQLEOFPacket::new(global_sequence_id);
                    let mut eof_payload = MySQLPacketPayload::new();
                    let eof_payload = DatabasePacket::encode(&mut eof_packet, &mut eof_payload);

                    payloads.push(eof_payload.get_payload());
                }
            },
            Statement::SetVariable{
                local:_,
                variable:_,
                value:_,} => {

                // This query will emit two result sets.
                let mut result = conn.query_iter(sql).unwrap();

                let global_sequence_id: u32 = 1;

                while let Some(result_set) = result.next_set() {
                    let result_set = result_set.unwrap();
                    let last_insert_id = match result_set.last_insert_id() {
                        Some(last_insert_id) => last_insert_id,
                        None => 0
                    };
                    let mut ok_packet = MySQLOKPacket::new(
                        global_sequence_id,
                        result_set.affected_rows(),
                        last_insert_id);
                    let mut ok_payload = MySQLPacketPayload::new();
                    let ok_payload = DatabasePacket::encode(&mut ok_packet, &mut ok_payload);

                    payloads.push(ok_payload.get_payload());
                }
            }

            _ => {}
        }
        Some(payloads)
    }
}

pub struct SetVariableHandler {}
impl CommandHandler<MySQLPacketPayload> for SetVariableHandler {
    fn handle(command_packet_header: Option<MySQLPacketHeader>, command_packet: Option<MySQLPacketPayload>) -> Option<Vec<Bytes>> {
        unimplemented!()
    }
}