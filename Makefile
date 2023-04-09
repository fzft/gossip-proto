release:
	cargo build

serve:
	./maelstrom/maelstrom serve --port 3000

echo: release
	./maelstrom/maelstrom test -w echo --bin target/debug/echo --node-count 1 --time-limit 10 --log-stderr


uid: release
	./maelstrom/maelstrom test -w unique-ids --bin target/debug/unique-ids --time-limit 30 --rate 1000 --node-count 3 --availability total --nemesis partition

broadcast: release
	./maelstrom/maelstrom test -w broadcast --bin target/debug/broadcast --node-count 1 --time-limit 20 --rate 10


# {"src": "c1", "dest": "n1", "body": {"type": "init", "node_id": "n3", "node_ids": ["n1","n2","n3"], "msg_id": 1}}
#{"src": "n1", "dest": "c1", "body" : {"msg_id": 0, "in_reply_to": 1, "type":"init_ok"}}