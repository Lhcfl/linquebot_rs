use rand::{seq::index, thread_rng, Rng};
use std::fmt;

pub struct MajorArcana {
    pub name: &'static str,
    pub cis: &'static str,
    pub trans: &'static str,
}

pub struct TarotChoosen {
    pub id: u8,
    pub is_reverse: bool,
    pub name: &'static str,
    pub description: &'static str,
}

impl fmt::Display for TarotChoosen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_reverse {
            write!(f, "{}逆位: {}", self.name, self.description)
        } else {
            write!(f, "{}正位: {}", self.name, self.description)
        }
    }
}

pub fn n_random_majors(n: usize) -> Vec<TarotChoosen> {
    assert!(n <= 22);
    index::sample(&mut thread_rng(), 21, n)
        .into_iter()
        .map(|id| {
            let rev: bool = thread_rng().gen();
            TarotChoosen {
                id: id as u8,
                is_reverse: rev,
                name: MAJORS[id].name,
                description: if rev {
                    MAJORS[id].trans
                } else {
                    MAJORS[id].cis
                },
            }
        })
        .collect::<Vec<_>>()
}

pub static MAJORS: &[MajorArcana] = &[
    MajorArcana {
        name: "【0】愚者（The Fool，0）",
        cis: "与众不同、幸运、不拘一格、追求新奇的梦想、眼界狭小、勇于冒险、向往自由、有艺术家气质、直攻要害、情感起伏不定、自由恋爱。",
        trans: "自负、固执、不安定、墨守成规、缺乏责任心、生活在梦幻中、不现实、不会应变、停滞不前、行为古怪、方向错误、感情不稳定。"
    },
    MajorArcana {
        name: "【1】魔术师（The Magician，I）",
        cis: "事情的开始，行动的改变，熟练的技术及技巧，贯彻我的意志，运用自然 的力量来达到野心。",
        trans: "意志力薄弱，起头难，走入错误的方向，知识不足，被骗和失败。"
    },
    MajorArcana {
        name: "【2】女祭司（The High Priestess，II）",
        cis: "开发出内在的神秘潜力，前途将有所变化的预言，深刻地思考，敏锐的洞察力，准确的直觉。",
        trans: "过于洁癖，无知，贪心，目光短浅，自尊心过高，偏差的判断，有勇无谋，自命不凡。"
    },
    MajorArcana {
        name: "【3】女皇（The Empress，III）",
        cis: "幸福，成功，收获，无忧无虑，圆满的家庭生活，良好的环境，美貌，艺术，与大自然接触，愉快的旅行，休闲。",
        trans: "不活泼，缺乏上进心，散漫的生活习惯，无法解决的事情，不能看到成果，担于享乐，环境险恶，与家人发生纠纷。"
    },
    MajorArcana {
        name: "【4】皇帝（The Emperor，IV）",
        cis: "光荣，权力，胜利，握有领导权，坚强的意志，达成目标，父亲的责任，精神上的孤单。",
        trans: "幼稚，无力，独裁，撒娇任性，平凡，没有自信，行动 力不足，意志薄弱，被支配。"
    },
    MajorArcana {
        name: "【4】皇帝（The Emperor，IV）",
        cis: "光荣，权力，胜利，握有领导权，坚强的意志，达成目标，父亲的责任，精神上的孤单。",
        trans: "幼稚，无力，独裁，撒娇任性，平凡，没有自信，行动力不足，意志薄弱，被支配。"
    },
    MajorArcana {
        name: "【5】教皇（The Hierophant，or the Pope，V）",
        cis: "援助，同情，宽宏大量，可信任的人给予的劝告，良好的商量对象。",
        trans: "错误的讯息，恶意的规劝，上当，援助被中断，愿望无法达成，被人利用，被放弃。"
    },
    MajorArcana {
        name: "【6】恋人（The Lovers，VI）",
        cis: "撮合，爱情，流行，兴趣，充满希望的未来，魅力，增加朋友。",
        trans: "禁不起诱惑，纵欲过度，反覆无常，友情变淡，厌倦，争吵，华丽的打扮，优柔寡断。"
    },
    MajorArcana {
        name: "【7】战车（The Chariot，VII）",
        cis: "努力而获得成功，胜利，克服障碍，行动力，自立，尝试，自我主张，年轻男子，交通工具，旅行运大吉。",
        trans: "争论失败，发生纠纷，阻滞，违返规则，诉诸暴力，顽固的男子，突然的失败，不良少年，挫折和自私自利。"
    },
    MajorArcana {
        name: "【8】力量（Strength，VIII）",
        cis: "大胆的行动 ，有勇气的决断，新发展，大转机，异动，以意志力战胜困难，健壮的女人。",
        trans: "胆小，输给强者，经不起诱惑，屈服在权威与常识之下，没有实践便告放弃，虚荣，懦弱，没有耐性。"
    },
    MajorArcana {
        name: "【9】隐者（The Hermit，IX）",
        cis: "隐藏的事实，个别的行动，倾听他人的意见，享受孤独，自己的丢化，有益的警戒，年长者，避开危险，祖父，乡间生活。",
        trans: "无视警，憎恨孤独，自卑，担心，幼稚思想，过于慎重导致失败，偏差，不宜旅行。"
    },
    MajorArcana {
        name: "【10】命运之轮（The Wheel of Fortune，X）",
        cis: "关键性的事件，有新的机会，因的潮流，环境的变化，幸运的开端，状况好转，问题解决，幸运之神降临。",
        trans: "边疆的不行，挫折，计划泡汤，障碍，无法修正方向，往坏处发展，恶性循环，中断。"
    },
    MajorArcana {
        name: "【11】正义（Justice，XI）",
        cis: "公正、中立、诚实、心胸坦荡、表里如一、身兼二职、追求 合理化、协调者、与法律有关、光明正大的交往、感情和睦。",
        trans: "失衡、偏见、纷扰、诉讼、独断专行、问心有愧、无法两全、表里不一、男女性格不合、情感波折、无视社会道德的恋情。"
    },
    MajorArcana {
        name: "【12】倒吊人（The Hanged Man，XII）",
        cis: "接受考验、行动受限、牺牲、不畏艰辛、不受利诱、有失必有得、吸取经验教训、浴火重生、广泛学习、奉献的爱。",
        trans: "无谓的牺牲、骨折、厄运、不够努力、处于劣势、任性、利己主义者、缺乏耐心、受惩罚、逃避爱情、没有结果的恋情。"
    },
    MajorArcana {
        name: "【13】 死神（Death，XIII）",
        cis: "失败、接近毁 灭、生病、失业、维持停滞状态、持续的损害、交易停止、枯燥的生活、别离、重新开始、双方有很深的鸿沟、恋情终止。",
        trans: "抱有一线希望、起死回生、回心转意、摆脱低迷状态、挽回名誉、身体康复、突然改变计划、逃避现实、斩断情丝、与旧情人相逢。"
    },
    MajorArcana {
        name: "【14】节制（Temperance，XIV）",
        cis: "单纯、调整、平顺、互惠互利、好感转为爱意、纯爱、深爱。",
        trans: "消耗、下降、疲劳、损失、不安、不融洽、爱情的配合度不佳。"
    },
    MajorArcana {
        name: "【15】恶魔（The Devil ，XV）",
        cis: "被束缚、堕落、生病、恶意、屈服、欲望的俘虏、不可抗拒的诱惑、颓废的生活、举债度日、不可告人的秘密、私密恋情。",
        trans: "逃离拘束、解除困扰、治愈病痛、告别过去、暂停、别离、拒绝诱惑、舍弃私欲、别离时刻、爱恨交加的恋情。"
    },
    MajorArcana {
        name: "【16】塔（The Tower，XVI）",
        cis: "破产、逆境、被开除、急病、致命的打击、巨大的 变动、受牵连、信念崩溃、玩火自焚、纷扰不断、突然分离，破灭的爱。",
        trans: "困境、内讧、紧迫的状态、状况不佳、趋于稳定、骄傲自大将付出代价、背水一战、分离的预感、爱情危机。"
    },
    MajorArcana {
        name: "【17】星星（The Star，XVII）",
        cis: "前途光明、充满希望、想象力、创造力、幻想、满足愿望、水准提高、理想的对象、美好的恋情。",
        trans: "挫折、失望、好高骛远、异想天开、仓皇失措、事与愿违、工作不顺心、情况悲观、秘密恋情、缺少爱的生活。"
    },
    MajorArcana {
        name: "【18】月亮（The Moon，XVIII）",
        cis: "不安、迷惑、动摇、谎言、欺骗、鬼迷心窍、动荡的爱、三角关系。",
        trans: "逃脱骗局、解除误会、状况好转、预知危险、等待、正视爱情的裂缝。"
    },
    MajorArcana {
        name: "【19】太阳（The Sun，XIX）",
        cis: "活跃、丰富的生命力、充满生机、精力充沛、工作顺利、贵人相助、幸福的婚姻、健康的交际。 ",
        trans: "消沉、体力不佳、缺乏连续性、意气消沉、生活不安、人际关系不好、感情波动、离婚。"
    },
    MajorArcana {
        name: "【20】审判（Judgement，XX）",
        cis: "复活的喜悦、康复、坦白、好消息、好运气、初露锋芒、复苏的爱、重逢、爱的奇迹。",
        trans: "一蹶不振 、幻灭、隐瞒、坏消息、无法决定、缺少目标、没有进展、消除、恋恋不舍。"
    },
    MajorArcana {
        name: "【21】世界（The World，XXI）",
        cis: "完成、成功、完美无缺、连续不断、精神亢奋、拥有毕生奋斗的目标、完成使命、幸运降临、快乐的结束、模范情侣。",
        trans: "未完成、失败、准备不足、盲目接受、一时不顺利、半途而废、精神颓废、饱和状态、合谋、态度不够融洽、感情受挫。"
    }
];
