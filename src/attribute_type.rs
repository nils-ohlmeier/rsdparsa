use std::fmt;
use std::net::IpAddr;

use SdpLine;
use error::SdpParserResult;
use network::{SdpAddrType, SdpNetType, parse_nettype, parse_addrtype, parse_unicast_addr, parse_unicast_addr_unknown_type};

#[derive(Clone)]
pub enum SdpAttributeType {
    // TODO consolidate these into groups
    BundleOnly,
    Candidate,
    EndOfCandidates,
    Extmap,
    Fingerprint,
    Fmtp,
    Group,
    IceLite,
    IceOptions,
    IcePwd,
    IceUfrag,
    Inactive,
    Mid,
    Msid,
    MsidSemantic,
    Rid,
    Recvonly,
    Rtcp,
    RtcpFb,
    RtcpMux,
    RtcpRsize,
    Rtpmap,
    Sctpmap,
    SctpPort,
    Sendonly,
    Sendrecv,
    Setup,
    Simulcast,
    Ssrc,
    SsrcGroup,
}

impl fmt::Display for SdpAttributeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let printable = match *self {
            SdpAttributeType::BundleOnly => "Bundle-Only",
            SdpAttributeType::Candidate => "Candidate",
            SdpAttributeType::EndOfCandidates => "End-Of-Candidates",
            SdpAttributeType::Extmap => "Extmap",
            SdpAttributeType::Fingerprint => "Fingerprint",
            SdpAttributeType::Fmtp => "Fmtp",
            SdpAttributeType::Group => "Group",
            SdpAttributeType::IceLite => "Ice-Lite",
            SdpAttributeType::IceOptions => "Ice-Options",
            SdpAttributeType::IcePwd => "Ice-Pwd",
            SdpAttributeType::IceUfrag => "Ice-Ufrag",
            SdpAttributeType::Inactive => "Inactive",
            SdpAttributeType::Mid => "Mid",
            SdpAttributeType::Msid => "Msid",
            SdpAttributeType::MsidSemantic => "Msid-Semantic",
            SdpAttributeType::Rid => "Rid",
            SdpAttributeType::Recvonly => "Recvonly",
            SdpAttributeType::Rtcp => "Rtcp",
            SdpAttributeType::RtcpFb => "Rtcp-Fb",
            SdpAttributeType::RtcpMux => "Rtcp-Mux",
            SdpAttributeType::RtcpRsize => "Rtcp-Rsize",
            SdpAttributeType::Rtpmap => "Rtpmap",
            SdpAttributeType::Sctpmap => "Sctpmap",
            SdpAttributeType::SctpPort => "Sctp-Port",
            SdpAttributeType::Sendonly => "Sendonly",
            SdpAttributeType::Sendrecv => "Sendrecv",
            SdpAttributeType::Setup => "Setup",
            SdpAttributeType::Simulcast => "Simulcast",
            SdpAttributeType::Ssrc => "Ssrc",
            SdpAttributeType::SsrcGroup => "Ssrc-Group",
        };
        write!(f, "{}", printable)
    }
}

#[derive(Clone)]
enum SdpAttributeCandidateTransport {
    Udp,
    Tcp
}

#[derive(Clone)]
enum SdpAttributeCandidateType {
    Host,
    Srflx,
    Prflx,
    Relay
}

#[derive(Clone)]
enum SdpAttributeCandidateTcpType {
    Active,
    Passive,
    Simultaneous
}

#[derive(Clone)]
struct SdpAttributeCandidate {
    foundation: String,
    component: u32,
    transport: SdpAttributeCandidateTransport,
    priority: u64,
    address: IpAddr,
    port: u32,
    c_type: SdpAttributeCandidateType,
    raddr: Option<IpAddr>,
    rport: Option<u32>,
    tcp_type: Option<SdpAttributeCandidateTcpType>
}

impl SdpAttributeCandidate {
    pub fn new(fd: String, comp: u32, transp: SdpAttributeCandidateTransport,
               prio: u64, addr: IpAddr, port: u32,
               ctyp: SdpAttributeCandidateType) -> SdpAttributeCandidate {
        SdpAttributeCandidate {
            foundation: fd,
            component: comp,
            transport: transp,
            priority: prio,
            address: addr,
            port: port,
            c_type: ctyp,
            raddr: None,
            rport: None,
            tcp_type: None
        }
    }

    fn set_remote_address(&mut self, ip: IpAddr) {
        self.raddr = Some(ip)
    }

    fn set_remote_port(&mut self, p: u32) {
        self.rport = Some(p)
    }

    fn set_tcp_type(&mut self, t: SdpAttributeCandidateTcpType) {
        self.tcp_type = Some(t)
    }
}

#[derive(Clone)]
struct SdpAttributeSimulcastId {
    id: String,
    paused: bool
}

impl SdpAttributeSimulcastId {
    pub fn new(idstr: String) -> SdpAttributeSimulcastId {
        if idstr.starts_with("~") {
            SdpAttributeSimulcastId {
                id: idstr[1..].to_string(),
                paused: true
            }
        } else {
            SdpAttributeSimulcastId {
                id: idstr,
                paused: false
            }
        }
    }
}

#[derive(Clone)]
struct SdpAttributeSimulcastAlternatives {
    ids: Vec<SdpAttributeSimulcastId>
}

impl SdpAttributeSimulcastAlternatives {
    pub fn new(idlist: String) -> SdpAttributeSimulcastAlternatives {
        SdpAttributeSimulcastAlternatives {
            ids: idlist.split(',')
                 .map(|x| x.to_string())
                 .map(|y| SdpAttributeSimulcastId::new(y))
                 .collect()
        }
    }
}

#[derive(Clone)]
struct SdpAttributeSimulcast {
    send: Vec<SdpAttributeSimulcastAlternatives>,
    receive: Vec<SdpAttributeSimulcastAlternatives>
}

impl SdpAttributeSimulcast {
    fn parse_ids(&mut self,
                 direction: SdpAttributeDirection,
                 idlist: String) {
        let list = idlist.split(';')
                   .map(|x| x.to_string())
                   .map(|y| SdpAttributeSimulcastAlternatives::new(y))
                   .collect();
        // TODO prevent over-writing existing values
        match direction {
            SdpAttributeDirection::Recvonly => self.receive = list,
            SdpAttributeDirection::Sendonly => self.send = list,
            _ => ()
        }
    }
}

#[derive(Clone)]
struct SdpAttributeRtcp {
    port: u32,
    nettype: SdpNetType,
    addrtype: SdpAddrType,
    unicast_addr: IpAddr
}

#[derive(Clone)]
struct SdpAttributeRtcpFb {
    payload_type: u32,
    // TODO parse this and use an enum instead?
    feedback_type: String
}

#[derive(Clone)]
enum SdpAttributeDirection {
    Recvonly,
    Sendonly,
    Sendrecv,
}

#[derive(Clone)]
struct SdpAttributeExtmap {
    id: u32,
    direction: Option<SdpAttributeDirection>,
    url: String
}

#[derive(Clone)]
struct SdpAttributeFmtp {
    payload_type: u32,
    tokens: Vec<String>
}

#[derive(Clone)]
struct SdpAttributeFingerprint {
    // TODO turn the supported hash algorithms into an enum?
    hash_algorithm: String,
    fingerprint: String
}

#[derive(Clone)]
struct SdpAttributeSctpmap {
    port: u32,
    channels: u32
}

#[derive(Clone)]
enum SdpAttributeGroupSemantic {
    LipSynchronization,
    FlowIdentification,
    SingleReservationFlow,
    AlternateNetworkAddressType,
    ForwardErrorCorrection,
    DecodingDependency,
    Bundle
}

#[derive(Clone)]
struct SdpAttributeGroup {
    semantics: SdpAttributeGroupSemantic,
    tags: Vec<String>
}

#[derive(Clone)]
struct SdpAttributeMsid {
    id: String,
    appdata: Option<String>
}

#[derive(Clone)]
struct SdpAttributeRtpmap {
    payload_type: u32,
    codec_name: String,
    frequency: Option<u32>,
    channels: Option<u32>
}

impl SdpAttributeRtpmap {
    pub fn new(pt: u32, codec: String) -> SdpAttributeRtpmap {
        SdpAttributeRtpmap { payload_type: pt,
                             codec_name: codec,
                             frequency: None,
                             channels: None
        }
    }

    fn set_frequency(&mut self, f: u32) {
        self.frequency = Some(f)
    }

    fn set_channels(&mut self, c: u32) {
        self.channels = Some(c)
    }
}

#[derive(Clone)]
enum SdpAttributeSetup {
    Active,
    Actpass,
    Holdconn,
    Passive
}

#[derive(Clone)]
struct SdpAttributeSsrc {
    id: u32,
    attribute: Option<String>,
    value: Option<String>
}

impl SdpAttributeSsrc {
    pub fn new(id: u32) -> SdpAttributeSsrc {
        SdpAttributeSsrc { id: id,
                           attribute: None,
                           value: None
        }
    }

    fn set_attribute(&mut self, a: &str) {
        if a.find(':') == None {
            self.attribute = Some(a.to_string());
        } else {
            let v: Vec<&str> = a.splitn(2, ':').collect();
            self.attribute = Some(v[0].to_string());
            self.value = Some(v[1].to_string());
        }
    }
}

#[derive(Clone)]
enum SdpAttributeValue {
    Str {value: String},
    Int {value: u32},
    Vector {value: Vec<String>},
    Candidate {value: SdpAttributeCandidate},
    Extmap {value: SdpAttributeExtmap},
    Fingerprint {value: SdpAttributeFingerprint},
    Fmtp {value: SdpAttributeFmtp},
    Group {value: SdpAttributeGroup},
    Msid {value: SdpAttributeMsid},
    Rtpmap {value: SdpAttributeRtpmap},
    Rtcp {value: SdpAttributeRtcp},
    Rtcpfb {value: SdpAttributeRtcpFb},
    Sctpmap {value: SdpAttributeSctpmap},
    Setup {value: SdpAttributeSetup},
    Simulcast {value: SdpAttributeSimulcast},
    Ssrc {value: SdpAttributeSsrc},
}

#[derive(Clone)]
pub struct SdpAttribute {
    name: SdpAttributeType,
    value: Option<SdpAttributeValue>
}

impl SdpAttribute {
    pub fn new(t: SdpAttributeType) -> SdpAttribute {
        SdpAttribute { name: t,
                       value: None
                     }
    }

    pub fn parse_value(&mut self, v: &str) -> Result<(), SdpParserResult> {
        match self.name {
            SdpAttributeType::BundleOnly |
            SdpAttributeType::EndOfCandidates |
            SdpAttributeType::IceLite |
            SdpAttributeType::Inactive |
            SdpAttributeType::Recvonly |
            SdpAttributeType::RtcpMux |
            SdpAttributeType::RtcpRsize |
            SdpAttributeType::Sendonly |
            SdpAttributeType::Sendrecv => {
                if v.len() >0 {
                    return Err(SdpParserResult::ParserLineError{
                        message: "This attribute is not allowed to have a value".to_string(),
                        line: v.to_string()})
                }
            },

            SdpAttributeType::IcePwd |
            SdpAttributeType::IceUfrag |
            SdpAttributeType::Mid |
            SdpAttributeType::Rid => {
                self.value = Some(SdpAttributeValue::Str {value: v.to_string()})
            },
            SdpAttributeType::MsidSemantic => {
                // mmusic-msid-16 no longer describes this...
                self.value = Some(SdpAttributeValue::Str {value: v.to_string()})
            },
            SdpAttributeType::SsrcGroup => {
                // JSEP has no support for it any more...
                self.value = Some(SdpAttributeValue::Str {value: v.to_string()})
            },

            SdpAttributeType::Candidate => {
                let tokens: Vec<&str> = v.split_whitespace().collect();
                if tokens.len() < 8 {
                    return Err(SdpParserResult::ParserLineError{
                        message: "Candidate needs to have minimum eigth tokens".to_string(),
                        line: v.to_string()})
                }
                let component = try!(tokens[1].parse::<u32>());
                let transport = match tokens[2].to_lowercase().as_ref() {
                    "udp" => SdpAttributeCandidateTransport::Udp,
                    "tcp" => SdpAttributeCandidateTransport::Tcp,
                    _ => return Err(SdpParserResult::ParserLineError{
                        message: "Unknonw candidate transport value".to_string(),
                        line: v.to_string()})
                };
                let priority = try!(tokens[3].parse::<u64>());
                let address = try!(parse_unicast_addr_unknown_type(tokens[4]));
                let port = try!(tokens[5].parse::<u32>());
                if port > 65535 {
                    return Err(SdpParserResult::ParserLineError{
                        message: "ICE candidate port can only be a bit 16bit number".to_string(),
                        line: v.to_string()})
                }
                match tokens[6].to_lowercase().as_ref() {
                    "typ" => (),
                    _ => return Err(SdpParserResult::ParserLineError{
                            message: "Candidate attribute token must be 'typ'".to_string(),
                            line: v.to_string()})
                };
                let cand_type = match tokens[7].to_lowercase().as_ref() {
                    "host" => SdpAttributeCandidateType::Host,
                    "srflx" => SdpAttributeCandidateType::Srflx,
                    "prflx" => SdpAttributeCandidateType::Prflx,
                    "relay" => SdpAttributeCandidateType::Relay,
                    _ => return Err(SdpParserResult::ParserLineError{
                            message: "Unknow candidate type value".to_string(),
                            line: v.to_string()})
                };
                let mut cand = SdpAttributeCandidate::new(tokens[0].to_string(),
                                                          component,
                                                          transport,
                                                          priority,
                                                          address,
                                                          port,
                                                          cand_type);
                if tokens.len() > 8 {
                    let mut index = 8;
                    while tokens.len() > index + 1 {
                        match tokens[index].to_lowercase().as_ref() {
                            "raddr" => {
                                let addr = try!(parse_unicast_addr_unknown_type(tokens[index + 1]));
                                cand.set_remote_address(addr);
                                index += 2;
                            },
                            "rport" => {
                                let port = try!(tokens[index + 1].parse::<u32>());
                                if port > 65535 {
                                    return Err(SdpParserResult::ParserLineError{
                                        message: "ICE candidate rport can only be a bit 16bit number".to_string(),
                                        line: v.to_string()})
                                }
                                cand.set_remote_port(port);
                                index += 2;
                            },
                            "tcptype" => {
                                cand.set_tcp_type(match tokens[index + 1].to_lowercase().as_ref() {
                                    "active" => SdpAttributeCandidateTcpType::Active,
                                    "passive" => SdpAttributeCandidateTcpType::Passive,
                                    "so" => SdpAttributeCandidateTcpType::Simultaneous,
                                    _ => return Err(SdpParserResult::ParserLineError{
                                        message: "Unknown tcptype value in candidate line".to_string(),
                                        line: v.to_string()})
                                });
                                index += 2;
                            },
                            _ => return Err(SdpParserResult::ParserUnsupported{
                                message: "Uknown candidate extension name".to_string(),
                                line: v.to_string()})
                        };
                    }
                }
                self.value = Some(SdpAttributeValue::Candidate {value:
                    cand
                })
            },
            SdpAttributeType::Extmap => {
                let tokens: Vec<&str> = v.split_whitespace().collect();
                if tokens.len() != 2 {
                    return Err(SdpParserResult::ParserLineError{
                        message: "Extmap needs to have two tokens".to_string(),
                        line: v.to_string()})
                }
                let id: u32;
                let mut dir: Option<SdpAttributeDirection> = None;
                if tokens[0].find('/') == None {
                    id = try!(tokens[0].parse::<u32>());
                } else {
                    let id_dir: Vec<&str> = tokens[0].splitn(2, '/').collect();
                    id = try!(id_dir[0].parse::<u32>());
                    dir = Some(match id_dir[1].to_lowercase().as_ref() {
                        "recvonly" => SdpAttributeDirection::Recvonly,
                        "sendonly" => SdpAttributeDirection::Sendonly,
                        "sendrecv" => SdpAttributeDirection::Sendrecv,
                        _ => return Err(SdpParserResult::ParserLineError{
                            message: "Unsupported direction in extmap value".to_string(),
                            line: v.to_string()}),
                    })
                }
                self.value = Some(SdpAttributeValue::Extmap {value:
                    SdpAttributeExtmap {
                        id: id,
                        direction: dir,
                        url: tokens[1].to_string()
                    }
                })
            },
            SdpAttributeType::Fingerprint => {
                let tokens: Vec<&str> = v.split_whitespace().collect();
                if tokens.len() != 2 {
                    return Err(SdpParserResult::ParserLineError{
                        message: "Fingerprint needs to have two tokens".to_string(),
                        line: v.to_string()})
                }
                self.value = Some(SdpAttributeValue::Fingerprint {value:
                    SdpAttributeFingerprint {
                        hash_algorithm: tokens[0].to_string(),
                        fingerprint: tokens[1].to_string()
                    }
                })
            },
            SdpAttributeType::Fmtp => {
                let tokens: Vec<&str> = v.split_whitespace().collect();
                if tokens.len() != 2 {
                    return Err(SdpParserResult::ParserLineError{
                        message: "Fmtp needs to have two tokens".to_string(),
                        line: v.to_string()})
                }
                self.value = Some(SdpAttributeValue::Fmtp {value:
                    SdpAttributeFmtp {
                        // TODO check for dynamic PT range
                        payload_type: try!(tokens[0].parse::<u32>()),
                        // TODO this should probably be slit into known tokens
                        // plus a list of unknown tokens
                        tokens: v.split(';').map(|x| x.to_string()).collect()
                    }
                })
            },
            SdpAttributeType::Group => {
                let mut tokens  = v.split_whitespace();
                let semantics = match tokens.next() {
                    None => return Err(SdpParserResult::ParserLineError{
                        message: "Group attribute is missing semantics token".to_string(),
                        line: v.to_string()}),
                    Some(x) =>  match x.to_uppercase().as_ref() {
                        "LS" => SdpAttributeGroupSemantic::LipSynchronization,
                        "FID" => SdpAttributeGroupSemantic::FlowIdentification,
                        "SRF" => SdpAttributeGroupSemantic::SingleReservationFlow,
                        "ANAT" => SdpAttributeGroupSemantic::AlternateNetworkAddressType,
                        "FEC" => SdpAttributeGroupSemantic::ForwardErrorCorrection,
                        "DDP" => SdpAttributeGroupSemantic::DecodingDependency,
                        "BUNDLE" => SdpAttributeGroupSemantic::Bundle,
                        _ => return Err(SdpParserResult::ParserLineError{
                            message: "Unsupported group semantics".to_string(),
                            line: v.to_string()}),
                    }
                };
                self.value = Some(SdpAttributeValue::Group {value:
                    SdpAttributeGroup {
                        semantics: semantics,
                        tags: tokens.map(|x| x.to_string()).collect()
                    }
                })
            },
            SdpAttributeType::IceOptions => {
                self.value = Some(SdpAttributeValue::Vector {
                    value: v.split_whitespace().map(|x| x.to_string()).collect()})
            },
            SdpAttributeType::Msid => {
                let mut tokens  = v.split_whitespace();
                let id = match tokens.next() {
                    None => return Err(SdpParserResult::ParserLineError{
                        message: "Msid attribute is missing msid-id token".to_string(),
                        line: v.to_string()}),
                    Some(x) => x.to_string()
                };
                let appdata = match tokens.next() {
                    None => None,
                    Some(x) => Some(x.to_string())
                };
                self.value = Some(SdpAttributeValue::Msid {value:
                    SdpAttributeMsid {
                        id: id,
                        appdata: appdata
                    }
                })
            },
            SdpAttributeType::Rtcp => {
                let tokens: Vec<&str> = v.split_whitespace().collect();
                if tokens.len() != 4 {
                    return Err(SdpParserResult::ParserLineError{
                        message: "Rtcp needs to have four tokens".to_string(),
                        line: v.to_string()})
                }
                let port = try!(tokens[0].parse::<u32>());
                if port > 65535 {
                    return Err(SdpParserResult::ParserLineError{
                        message: "Rtcp port can only be a bit 16bit number".to_string(),
                        line: v.to_string()})
                }
                let nettype = try!(parse_nettype(tokens[1]));
                let addrtype = try!(parse_addrtype(tokens[2]));
                let unicast_addr = try!(parse_unicast_addr(&addrtype, tokens[3]));
                self.value = Some(SdpAttributeValue::Rtcp {value:
                    SdpAttributeRtcp {
                        port: port,
                        nettype: nettype,
                        addrtype: addrtype,
                        unicast_addr: unicast_addr
                    }
                })
            },
            SdpAttributeType::RtcpFb => {
                let tokens: Vec<&str> = v.splitn(2, ' ').collect();
                self.value = Some(SdpAttributeValue::Rtcpfb {value:
                    SdpAttributeRtcpFb {
                        // TODO limit this to dymaic PTs
                        payload_type: try!(tokens[0].parse::<u32>()),
                        feedback_type: tokens[1].to_string()
                    }
                });
            },
            SdpAttributeType::Rtpmap => {
                let tokens: Vec<&str> = v.split_whitespace().collect();
                if tokens.len() != 2 {
                    return Err(SdpParserResult::ParserLineError{
                        message: "Rtpmap needs to have two tokens".to_string(),
                        line: v.to_string()})
                }
                // TODO limit this to dymaic PTs
                let payload_type: u32 = try!(tokens[0].parse::<u32>());
                let split: Vec<&str> = tokens[1].split('/').collect();
                if split.len() > 3 {
                    return Err(SdpParserResult::ParserLineError{
                        message: "Rtpmap codec token can max 3 subtokens".to_string(),
                        line: v.to_string()})
                }
                let mut rtpmap = SdpAttributeRtpmap::new(payload_type,
                                                         split[0].to_string());
                if split.len() > 1 {
                    rtpmap.set_frequency(try!(split[1].parse::<u32>()));
                }
                if split.len() > 2 {
                    rtpmap.set_channels(try!(split[2].parse::<u32>()));
                }
                self.value = Some(SdpAttributeValue::Rtpmap {value: rtpmap})
            },
            SdpAttributeType::Sctpmap => {
                let tokens: Vec<&str> = v.split_whitespace().collect();
                if tokens.len() != 3 {
                    return Err(SdpParserResult::ParserLineError{
                        message: "Sctpmap needs to have three tokens".to_string(),
                        line: v.to_string()})
                }
                let port = try!(tokens[0].parse::<u32>());
                if port > 65535 {
                    return Err(SdpParserResult::ParserLineError{
                        message: "Sctpmap port can only be a bit 16bit number".to_string(),
                        line: v.to_string()})
                }
                if tokens[1].to_lowercase() != "webrtc-datachannel" {
                    return Err(SdpParserResult::ParserLineError{
                        message: "Unsupported sctpmap type token".to_string(),
                        line: v.to_string()})
                }
                self.value = Some(SdpAttributeValue::Sctpmap {value:
                    SdpAttributeSctpmap {
                        port: port,
                        channels: try!(tokens[2].parse::<u32>())
                    }
                });
            },
            SdpAttributeType::SctpPort => {
                let port = try!(v.parse::<u32>());
                if port > 65535 {
                    return Err(SdpParserResult::ParserLineError{
                        message: "Sctpport port can only be a bit 16bit number".to_string(),
                        line: v.to_string()})
                }
                self.value = Some(SdpAttributeValue::Int {
                    value: port
                })
            }
            SdpAttributeType::Simulcast => {
                let mut tokens = v.split_whitespace();
                let mut token = match tokens.next() {
                    None => return Err(SdpParserResult::ParserLineError{
                        message: "Simulcast attribute is missing send/recv value".to_string(),
                        line: v.to_string()}),
                    Some(x) => x,
                };
                let mut sc = SdpAttributeSimulcast {
                    send: Vec::new(),
                    receive: Vec::new()
                };
                loop {
                    let sendrecv = match token.to_lowercase().as_ref() {
                        "send" => SdpAttributeDirection::Sendonly,
                        "recv" => SdpAttributeDirection::Recvonly,
                        _ => return Err(SdpParserResult::ParserLineError{
                        message: "Unsupported send/recv value in simulcast attribute".to_string(),
                        line: v.to_string()}),
                    };
                    match tokens.next() {
                        None => return Err(SdpParserResult::ParserLineError{
                            message: "Simulcast attribute is missing id list".to_string(),
                            line: v.to_string()}),
                        Some(x) => sc.parse_ids(sendrecv, x.to_string()),
                    };
                    token = match tokens.next() {
                        None => { break; },
                        Some(x) => x,
                    };
                }
                self.value = Some(SdpAttributeValue::Simulcast {
                    value: sc
                })
            },
            SdpAttributeType::Setup => {
                self.value = Some(SdpAttributeValue::Setup {value:
                    match v.to_lowercase().as_ref() {
                        "active" => SdpAttributeSetup::Active,
                        "actpass" => SdpAttributeSetup::Actpass,
                        "holdconn" => SdpAttributeSetup::Holdconn,
                        "passive" => SdpAttributeSetup::Passive,
                        _ => return Err(SdpParserResult::ParserLineError{
                            message: "Unsupported setup value".to_string(),
                            line: v.to_string()}),
                    }
                })
            },
            SdpAttributeType::Ssrc => {
                let mut tokens  = v.split_whitespace();
                let ssrc_id = match tokens.next() {
                    None => return Err(SdpParserResult::ParserLineError{
                        message: "Ssrc attribute is missing ssrc-id value".to_string(),
                        line: v.to_string()}),
                    Some(x) => try!(x.parse::<u32>())
                };
                let mut ssrc = SdpAttributeSsrc::new(ssrc_id);
                match tokens.next() {
                    None => (),
                    Some(x) => ssrc.set_attribute(x),
                };
                self.value = Some(SdpAttributeValue::Ssrc {
                    value: ssrc
                })
            },
        }
        Ok(())
    }
}

pub fn parse_attribute(value: &str) -> Result<SdpLine, SdpParserResult> {
    let name: &str;
    let mut val: &str = "";
    if value.find(':') == None {
        name = value;
    } else {
        let v: Vec<&str> = value.splitn(2, ':').collect();
        name = v[0];
        val = v[1];
    }
    let attrtype = match name.to_lowercase().as_ref() {
        "bundle-only" => SdpAttributeType::BundleOnly,
        "candidate" => SdpAttributeType::Candidate,
        "end-of-candidates" => SdpAttributeType::EndOfCandidates,
        "extmap" => SdpAttributeType::Extmap,
        "fingerprint" => SdpAttributeType::Fingerprint,
        "fmtp" => SdpAttributeType::Fmtp,
        "group" => SdpAttributeType::Group,
        "ice-lite" => SdpAttributeType::IceLite,
        "ice-options" => SdpAttributeType::IceOptions,
        "ice-pwd" => SdpAttributeType::IcePwd,
        "ice-ufrag" => SdpAttributeType::IceUfrag,
        "inactive" => SdpAttributeType::Inactive,
        "mid" => SdpAttributeType::Mid,
        "msid" => SdpAttributeType::Msid,
        "msid-semantic" => SdpAttributeType::MsidSemantic,
        "rid" => SdpAttributeType::Rid,
        "recvonly" => SdpAttributeType::Recvonly,
        "rtcp" => SdpAttributeType::Rtcp,
        "rtcp-fb" => SdpAttributeType::RtcpFb,
        "rtcp-mux" => SdpAttributeType::RtcpMux,
        "rtcp-rsize" => SdpAttributeType::RtcpRsize,
        "rtpmap" => SdpAttributeType::Rtpmap,
        "sctpmap" => SdpAttributeType::Sctpmap,
        "sctp-port" => SdpAttributeType::SctpPort,
        "sendonly" => SdpAttributeType::Sendonly,
        "sendrecv" => SdpAttributeType::Sendrecv,
        "setup" => SdpAttributeType::Setup,
        "simulcast" => SdpAttributeType::Simulcast,
        "ssrc" => SdpAttributeType::Ssrc,
        "ssrc-group" => SdpAttributeType::SsrcGroup,
        _ => return Err(SdpParserResult::ParserUnsupported {
              message: "unsupported attribute value".to_string(),
              line: name.to_string() }),
    };
    let mut attr = SdpAttribute::new(attrtype);
    try!(attr.parse_value(val.trim()));
    /*
    println!("attribute: {}, {}", 
             a.name, a.value.some());
             */
    Ok(SdpLine::Attribute { value: attr })
}

#[test]
fn test_parse_attribute_candidate() {
    assert!(parse_attribute("candidate:0 1 UDP 2122252543 172.16.156.106 49760 typ host").is_ok());
    assert!(parse_attribute("candidate:foo 1 UDP 2122252543 172.16.156.106 49760 typ host").is_ok());
    assert!(parse_attribute("candidate:0 1 TCP 2122252543 172.16.156.106 49760 typ host").is_ok());
    assert!(parse_attribute("candidate:0 1 TCP 2122252543 ::1 49760 typ host").is_ok());
    assert!(parse_attribute("candidate:0 1 UDP 2122252543 172.16.156.106 49760 typ srflx").is_ok());
    assert!(parse_attribute("candidate:0 1 UDP 2122252543 172.16.156.106 49760 typ prflx").is_ok());
    assert!(parse_attribute("candidate:0 1 UDP 2122252543 172.16.156.106 49760 typ relay").is_ok());
    assert!(parse_attribute("candidate:0 1 TCP 2122252543 172.16.156.106 49760 typ host tcptype active").is_ok());
    assert!(parse_attribute("candidate:0 1 TCP 2122252543 172.16.156.106 49760 typ host tcptype passive").is_ok());
    assert!(parse_attribute("candidate:0 1 TCP 2122252543 172.16.156.106 49760 typ host tcptype so").is_ok());
    assert!(parse_attribute("candidate:1 1 UDP 1685987071 24.23.204.141 54609 typ srflx raddr 192.168.1.4 rport 61665").is_ok());
    assert!(parse_attribute("candidate:1 1 TCP 1685987071 24.23.204.141 54609 typ srflx raddr 192.168.1.4 rport 61665 tcptype passive").is_ok());

    assert!(parse_attribute("candidate:0 1 UDP 2122252543 172.16.156.106 49760 typ").is_err());
    assert!(parse_attribute("candidate:0 foo UDP 2122252543 172.16.156.106 49760 typ host").is_err());
    assert!(parse_attribute("candidate:0 1 FOO 2122252543 172.16.156.106 49760 typ host").is_err());
    assert!(parse_attribute("candidate:0 1 UDP foo 172.16.156.106 49760 typ host").is_err());
    assert!(parse_attribute("candidate:0 1 UDP 2122252543 172.16.156 49760 typ host").is_err());
    assert!(parse_attribute("candidate:0 1 UDP 2122252543 172.16.156.106 70000 typ host").is_err());
    assert!(parse_attribute("candidate:0 1 UDP 2122252543 172.16.156.106 49760 type host").is_err());
    assert!(parse_attribute("candidate:0 1 UDP 2122252543 172.16.156.106 49760 typ fost").is_err());
    assert!(parse_attribute("candidate:1 1 UDP 1685987071 24.23.204.141 54609 typ srflx raddr 192.168.1 rport 61665").is_err());
    assert!(parse_attribute("candidate:1 1 UDP 1685987071 24.23.204.141 54609 typ srflx raddr 192.168.1.4 rport 70000").is_err());
}

#[test]
fn test_parse_attribute_end_of_candidates() {
    assert!(parse_attribute("end-of-candidates").is_ok())
}

#[test]
fn test_parse_attribute_extmap() {
    assert!(parse_attribute("extmap:1/sendonly urn:ietf:params:rtp-hdrext:ssrc-audio-level").is_ok());
    assert!(parse_attribute("extmap:3 http://www.webrtc.org/experiments/rtp-hdrext/abs-send-time").is_ok());
}

#[test]
fn test_parse_attribute_fingerprint() {
    assert!(parse_attribute("fingerprint:sha-256 CD:34:D1:62:16:95:7B:B7:EB:74:E2:39:27:97:EB:0B:23:73:AC:BC:BF:2F:E3:91:CB:57:A9:9D:4A:A2:0B:40").is_ok())
}

#[test]
fn test_parse_attribute_fmtp() {
    assert!(parse_attribute("fmtp:109 maxplaybackrate=48000;stereo=1;useinbandfec=1").is_ok())
}

#[test]
fn test_parse_attribute_group() {
    assert!(parse_attribute("group:LS").is_ok());
    assert!(parse_attribute("group:LS 1 2").is_ok());
    assert!(parse_attribute("group:BUNDLE sdparta_0 sdparta_1 sdparta_2").is_ok());

    assert!(parse_attribute("group:").is_err());
    assert!(parse_attribute("group:NEVER_SUPPORTED_SEMANTICS").is_err());
}

#[test]
fn test_parse_attribute_bundle_only() {
    assert!(parse_attribute("bundle-only").is_ok())
}

#[test]
fn test_parse_attribute_ice_lite() {
    assert!(parse_attribute("ice-lite").is_ok())
}

#[test]
fn test_parse_attribute_ice_options() {
    assert!(parse_attribute("ice-options:trickle").is_ok())
}

#[test]
fn test_parse_attribute_ice_pwd() {
    assert!(parse_attribute("ice-pwd:e3baa26dd2fa5030d881d385f1e36cce").is_ok())
}

#[test]
fn test_parse_attribute_ice_ufrag() {
    assert!(parse_attribute("ice-ufrag:58b99ead").is_ok())
}

#[test]
fn test_parse_attribute_inactive() {
    assert!(parse_attribute("inactive").is_ok())
}

#[test]
fn test_parse_attribute_mid() {
    assert!(parse_attribute("mid:sdparta_0").is_ok())
}

#[test]
fn test_parse_attribute_msid() {
    assert!(parse_attribute("msid:{5a990edd-0568-ac40-8d97-310fc33f3411}").is_ok());
    assert!(parse_attribute("msid:{5a990edd-0568-ac40-8d97-310fc33f3411} {218cfa1c-617d-2249-9997-60929ce4c405}").is_ok());

    assert!(parse_attribute("msid:").is_err());
}

#[test]
fn test_parse_attribute_msid_semantics() {
    assert!(parse_attribute("msid-semantic:WMS *").is_ok())
}

#[test]
fn test_parse_attribute_rid() {
    assert!(parse_attribute("rid:foo send").is_ok())
}

#[test]
fn test_parse_attribute_recvonly() {
    assert!(parse_attribute("recvonly").is_ok())
}

#[test]
fn test_parse_attribute_rtcp() {
    assert!(parse_attribute("rtcp:9 IN IP4 0.0.0.0").is_ok())
}

#[test]
fn test_parse_attribute_rtcp_fb() {
    assert!(parse_attribute("rtcp-fb:101 ccm fir").is_ok())
}

#[test]
fn test_parse_attribute_rtcp_mux() {
    assert!(parse_attribute("rtcp-mux").is_ok())
}

#[test]
fn test_parse_attribute_rtcp_rsize() {
    assert!(parse_attribute("rtcp-rsize").is_ok())
}

#[test]
fn test_parse_attribute_rtpmap() {
    assert!(parse_attribute("rtpmap:109 opus/48000/2").is_ok())
}

#[test]
fn test_parse_attribute_sctpmap() {
    assert!(parse_attribute("sctpmap:5000 webrtc-datachannel 256").is_ok())
}

#[test]
fn test_parse_attribute_sctp_port() {
    assert!(parse_attribute("sctp-port:5000").is_ok())
}

#[test]
fn test_parse_attribute_simulcast() {
    assert!(parse_attribute("simulcast:send 1").is_ok());
    assert!(parse_attribute("simulcast:recv test").is_ok());
    assert!(parse_attribute("simulcast:recv ~test").is_ok());
    assert!(parse_attribute("simulcast:recv test;foo").is_ok());
    assert!(parse_attribute("simulcast:recv foo,bar").is_ok());
    assert!(parse_attribute("simulcast:recv foo,bar;test").is_ok());
    assert!(parse_attribute("simulcast:recv 1;4,5 send 6;7").is_ok());
    assert!(parse_attribute("simulcast:send 1,2,3;~4,~5 recv 6;~7,~8").is_ok());
    // old draft 03 notation used by Firefox 55
    assert!(parse_attribute("simulcast: send rid=foo;bar").is_ok());

    assert!(parse_attribute("simulcast:send").is_err());
    assert!(parse_attribute("simulcast:foobar 1").is_err());
    assert!(parse_attribute("simulcast:send 1 foobar 2").is_err());
}

#[test]
fn test_parse_attribute_ssrc() {
    assert!(parse_attribute("ssrc:2655508255").is_ok());
    assert!(parse_attribute("ssrc:2655508255 foo").is_ok());
    assert!(parse_attribute("ssrc:2655508255 cname:{735484ea-4f6c-f74a-bd66-7425f8476c2e}").is_ok());

    assert!(parse_attribute("ssrc:").is_err());
    assert!(parse_attribute("ssrc:foo").is_err());
}

#[test]
fn test_parse_attribute_ssrc_group() {
    assert!(parse_attribute("ssrc-group:FID 3156517279 2673335628").is_ok())
}
