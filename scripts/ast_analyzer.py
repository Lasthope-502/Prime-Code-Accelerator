import ast
import json
import sys


class PatternDetector(ast.NodeVisitor):
    def __init__(self):
        self.matches = []

    def is_range_call(self, node):
        return (isinstance(node, ast.Call)
                and isinstance(node.func, ast.Name)
                and node.func.id == 'range')

    def visit_For(self, node):
        self.check_sum_of_squares(node)
        self.check_range_sum(node)
        self.check_nested_loop(node)
        self.check_string_concat(node)
        self.check_list_append(node)
        self.check_dict_counting(node)
        self.generic_visit(node)

    def check_sum_of_squares(self, node):
        if not self.is_range_call(node.iter) or not isinstance(node.target, ast.Name):
            return
        var = node.target.id
        for stmt in node.body:
            if isinstance(stmt, ast.AugAssign) and isinstance(stmt.op, ast.Add):
                v = stmt.value
                if isinstance(v, ast.BinOp) and isinstance(v.op, ast.Mult):
                    l, r = v.left, v.right
                    if (isinstance(l, ast.Name) and l.id == var and
                            isinstance(r, ast.Name) and r.id == var):
                        self.matches.append({
                            "name": "sum_of_squares_loop",
                            "description": f"'{var} * {var}' accumulation pattern (AST-verified)",
                            "rust_fn": "sum_of_squares",
                            "category": "numeric_loop",
                            "line": node.lineno,
                        })

    def check_range_sum(self, node):
        if not self.is_range_call(node.iter) or not isinstance(node.target, ast.Name):
            return
        var = node.target.id
        for stmt in node.body:
            if isinstance(stmt, ast.AugAssign) and isinstance(stmt.op, ast.Add):
                if isinstance(stmt.value, ast.Name) and stmt.value.id == var:
                    self.matches.append({
                        "name": "range_sum_loop",
                        "description": f"simple '+= {var}' accumulation loop (AST-verified)",
                        "rust_fn": "fast_range_sum",
                        "category": "numeric_loop",
                        "line": node.lineno,
                    })

    def check_nested_loop(self, node):
        for stmt in node.body:
            if isinstance(stmt, ast.For):
                self.matches.append({
                    "name": "nested_loop_matrix",
                    "description": "Nested for-loops detected (AST-verified) — O(n^2)+ candidate",
                    "rust_fn": "matrix_multiply",
                    "category": "nested_loop",
                    "line": node.lineno,
                })
                break

    def check_string_concat(self, node):
        for stmt in node.body:
            if isinstance(stmt, ast.AugAssign) and isinstance(stmt.op, ast.Add):
                v = stmt.value
                is_stringy = (
                    isinstance(v, ast.JoinedStr) or
                    (isinstance(v, ast.Constant) and isinstance(v.value, str)) or
                    (isinstance(v, ast.Call) and isinstance(v.func, ast.Name) and v.func.id == 'str')
                )
                if is_stringy:
                    self.matches.append({
                        "name": "string_concat_loop",
                        "description": "String += concatenation in loop (AST-verified, O(n^2) risk)",
                        "rust_fn": "fast_string_join",
                        "category": "string_ops",
                        "line": node.lineno,
                    })

    def check_list_append(self, node):
        for sub in ast.walk(node):
            if (isinstance(sub, ast.Call) and isinstance(sub.func, ast.Attribute)
                    and sub.func.attr == 'append'):
                self.matches.append({
                    "name": "list_append_loop",
                    "description": "list.append() inside loop (AST-verified)",
                    "rust_fn": "fast_collect",
                    "category": "collection_ops",
                    "line": node.lineno,
                })
                break

    def check_dict_counting(self, node):
        """
        Detects genuine dict frequency-counting patterns like:
            counts[word] += 1
            counts[word] = counts.get(word, 0) + 1

        Explicitly EXCLUDES nested subscripts like result[i][j] += x
        (that's a matrix access pattern, not dict counting) by requiring
        the subscript's base value to be a plain Name (not another Subscript).
        """
        for sub in ast.walk(node):
            if isinstance(sub, ast.AugAssign) and isinstance(sub.target, ast.Subscript):
                target = sub.target
                # Reject nested subscripts: result[i][j] -> target.value is itself a Subscript
                if isinstance(target.value, ast.Subscript):
                    continue
                # Require target.value to be a simple Name (e.g. "counts")
                if not isinstance(target.value, ast.Name):
                    continue
                # Require the increment amount to be a small constant (typical for counting)
                if isinstance(sub.op, ast.Add) and isinstance(sub.value, ast.Constant):
                    if isinstance(sub.value.value, (int, float)):
                        self.matches.append({
                            "name": "dict_counting_loop",
                            "description": f"Dict subscript '{target.value.id}[...] += {sub.value.value}' pattern (AST-verified, frequency counting)",
                            "rust_fn": "fast_word_count",
                            "category": "collection_ops",
                            "line": node.lineno,
                        })
                        break


def find_enclosing_scope(tree, line_no):
    best = tree
    best_line = 0
    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            start = node.lineno
            end = getattr(node, 'end_lineno', start)
            if start <= line_no <= end and start > best_line:
                best = node
                best_line = start
    return best


def main():
    if len(sys.argv) < 3:
        print(json.dumps([]))
        return

    filepath = sys.argv[1]
    line_no = int(sys.argv[2])

    try:
        with open(filepath) as f:
            source = f.read()
        tree = ast.parse(source, filepath)
    except (SyntaxError, FileNotFoundError):
        print(json.dumps([]))
        return

    scope = find_enclosing_scope(tree, line_no)

    detector = PatternDetector()
    detector.visit(scope)

    seen = set()
    unique = []
    for m in detector.matches:
        key = (m['name'], m['line'])
        if key not in seen:
            seen.add(key)
            unique.append(m)

    print(json.dumps(unique))


if __name__ == "__main__":
    main()