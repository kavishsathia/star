// expect: 1
// expect: 2
struct Node {
    next: Node?,
    content: integer
}

fn main(): integer {
    let list: Node = new Node {
        next: new Node {
            next: new Node {
                next: null,
                content: 0
            }
            content: 2
        }
        content: 1
    };

    while list.content != 0 {
        print $list.content;
        list = list.next??;
    }

    return 0;
}
