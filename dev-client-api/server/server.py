#!/usr/bin/env python3
"""
HTTP Server for Axicor Visualizer.

Serves the visualizer web pages and provides endpoints for saving
visual layout overrides (shard positions, socket pitch/dimensions/offsets),
as well as endpoints for project listing, loading, and script imports.
"""

import json
import os
import sys
from http.server import SimpleHTTPRequestHandler, HTTPServer

# Ensure root directory is in python path
ROOT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if ROOT_DIR not in sys.path:
    sys.path.insert(0, ROOT_DIR)

from persistence import update_and_regenerate

PORT = 8080
STATIC_DIR = os.path.abspath(os.path.join(ROOT_DIR, "..", "dev-js-api"))

class VisualizerHandler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        # Serve from the static dev-js-api directory
        super().__init__(*args, directory=STATIC_DIR, **kwargs)

    def translate_path(self, path):
        import urllib.parse
        from persistence import SCRIPTS_DIR, LOCAL_DIR, MODELS_DIR
        
        # Decode path first
        path_norm = urllib.parse.unquote(path)
        
        # Intercept and route /projects/local/ -> .local-storage/
        if path_norm.startswith("/projects/local/") or path_norm.startswith("projects/local/"):
            prefix = "/projects/local/" if path_norm.startswith("/") else "projects/local/"
            rel_path = path_norm[len(prefix):]
            rel_path = rel_path.replace("/", os.sep)
            return os.path.join(LOCAL_DIR, rel_path)
            
        # Intercept and route /projects/scripts/ -> examples/
        elif path_norm.startswith("/projects/scripts/") or path_norm.startswith("projects/scripts/"):
            prefix = "/projects/scripts/" if path_norm.startswith("/") else "projects/scripts/"
            rel_path = path_norm[len(prefix):]
            rel_path = rel_path.replace("/", os.sep)
            return os.path.join(SCRIPTS_DIR, rel_path)
            
        # Intercept and route /projects/models/ -> models/
        elif path_norm.startswith("/projects/models/") or path_norm.startswith("projects/models/"):
            prefix = "/projects/models/" if path_norm.startswith("/") else "projects/models/"
            rel_path = path_norm[len(prefix):]
            rel_path = rel_path.replace("/", os.sep)
            return os.path.join(MODELS_DIR, rel_path)
            
        return super().translate_path(path)

    def end_headers(self):
        self.send_header("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0")
        self.send_header("Pragma", "no-cache")
        self.send_header("Expires", "0")
        super().end_headers()

    def do_GET(self):
        if self.path == "/api/projects":
            self.handle_list_projects()
        elif self.path == "/api/dev/status":
            self.handle_dev_status()
        else:
            super().do_GET()

    def do_POST(self):
        if self.path == "/api/save":
            self.handle_save()
        elif self.path == "/api/projects/load":
            self.handle_load_project()
        elif self.path == "/api/projects/import":
            self.handle_import_project()
        elif self.path == "/api/projects/rename":
            self.handle_rename_project()
        elif self.path == "/api/projects/delete":
            self.handle_delete_project()
        elif self.path == "/api/projects/create":
            self.handle_create_project()
        elif self.path == "/api/dev/command":
            self.handle_dev_command()
        else:
            self.send_error(404, "Endpoint not found")

    def handle_list_projects(self):
        try:
            from persistence import list_projects
            data = list_projects()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(json.dumps(data).encode("utf-8"))
        except Exception as e:
            print(f"Error in handle_list_projects: {e}")
            self.send_response(500)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            response = {"status": "error", "message": str(e)}
            self.wfile.write(json.dumps(response).encode("utf-8"))

    def handle_load_project(self):
        try:
            content_length = int(self.headers.get("Content-Length", 0))
            post_data = self.rfile.read(content_length)
            payload = json.loads(post_data.decode("utf-8"))
            
            proj_type = payload.get("type")
            name = payload.get("name")
            
            from persistence import compile_project
            
            if proj_type == "script":
                project_name = name.rsplit('.', 1)[0]
                compile_project(project_name, name)
                response = {"status": "success", "project": project_name}
            elif proj_type == "local":
                response = {"status": "success", "project": name}
            else:
                response = {"status": "error", "message": f"Unsupported project type: {proj_type}"}
                
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(json.dumps(response).encode("utf-8"))
        except Exception as e:
            print(f"Error in handle_load_project: {e}")
            self.send_response(500)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            response = {"status": "error", "message": str(e)}
            self.wfile.write(json.dumps(response).encode("utf-8"))

    def handle_import_project(self):
        try:
            content_length = int(self.headers.get("Content-Length", 0))
            post_data = self.rfile.read(content_length)
            payload = json.loads(post_data.decode("utf-8"))
            
            filename = payload.get("filename")
            content = payload.get("content")
            
            if not filename or not content:
                raise ValueError("Missing filename or content")
                
            from persistence import SCRIPTS_DIR
            target_path = os.path.join(SCRIPTS_DIR, filename)
                
            with open(target_path, "w", encoding="utf-8") as f:
                f.write(content)
                
            print(f"Imported project file saved to {target_path}")
            
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            response = {"status": "success", "message": f"Imported {filename} successfully"}
            self.wfile.write(json.dumps(response).encode("utf-8"))
        except Exception as e:
            print(f"Error in handle_import_project: {e}")
            self.send_response(500)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            response = {"status": "error", "message": str(e)}
            self.wfile.write(json.dumps(response).encode("utf-8"))

    def handle_save(self):
        try:
            content_length = int(self.headers.get("Content-Length", 0))
            post_data = self.rfile.read(content_length)
            payload = json.loads(post_data.decode("utf-8"))

            # Update and regenerate layout/routes
            update_and_regenerate(payload)

            # Respond success
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            
            response = {"status": "success", "message": "Layout saved and regenerated"}
            self.wfile.write(json.dumps(response).encode("utf-8"))

        except Exception as e:
            print(f"Error in handle_save: {e}")
            self.send_response(500)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            response = {"status": "error", "message": str(e)}
            self.wfile.write(json.dumps(response).encode("utf-8"))

    def handle_rename_project(self):
        try:
            content_length = int(self.headers.get("Content-Length", 0))
            post_data = self.rfile.read(content_length)
            payload = json.loads(post_data.decode("utf-8"))
            
            old_name = payload.get("oldName")
            new_name = payload.get("newName")
            
            if not old_name or not new_name:
                raise ValueError("Missing oldName or newName")
                
            from persistence import rename_project
            rename_project(old_name, new_name)
            
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            response = {"status": "success", "message": f"Renamed {old_name} to {new_name}"}
            self.wfile.write(json.dumps(response).encode("utf-8"))
        except Exception as e:
            print(f"Error in handle_rename_project: {e}")
            self.send_response(500)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            response = {"status": "error", "message": str(e)}
            self.wfile.write(json.dumps(response).encode("utf-8"))

    def handle_delete_project(self):
        try:
            content_length = int(self.headers.get("Content-Length", 0))
            post_data = self.rfile.read(content_length)
            payload = json.loads(post_data.decode("utf-8"))
            
            name = payload.get("name")
            
            if not name:
                raise ValueError("Missing project name")
                
            from persistence import delete_project
            delete_project(name)
            
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            response = {"status": "success", "message": f"Deleted project {name}"}
            self.wfile.write(json.dumps(response).encode("utf-8"))
        except Exception as e:
            print(f"Error in handle_delete_project: {e}")
            self.send_response(500)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            response = {"status": "error", "message": str(e)}
            self.wfile.write(json.dumps(response).encode("utf-8"))

    def handle_create_project(self):
        try:
            content_length = int(self.headers.get("Content-Length", 0))
            post_data = self.rfile.read(content_length)
            payload = json.loads(post_data.decode("utf-8"))
            
            name = payload.get("name")
            if not name:
                raise ValueError("Missing project name")
                
            from persistence import create_project
            create_project(name)
            
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            response = {"status": "success", "message": f"Created project {name}"}
            self.wfile.write(json.dumps(response).encode("utf-8"))
        except Exception as e:
            print(f"Error in handle_create_project: {e}")
            self.send_response(500)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            response = {"status": "error", "message": str(e)}
            self.wfile.write(json.dumps(response).encode("utf-8"))

    def handle_dev_status(self):
        try:
            import sys
            import os
            import importlib.util
            workspace_root = os.path.abspath(os.path.join(ROOT_DIR, ".."))
            execute_path = os.path.join(workspace_root, ".AxiConsole", "execute.py")
            
            spec = importlib.util.spec_from_file_location("execute", execute_path)
            execute = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(execute)
            
            # Server status
            pid = execute.get_running_pid()
            server_status = "running" if pid else "stopped"
            
            # Rust status
            rust_built = False
            target_dir = os.path.join(workspace_root, "dev-rust-api", "target", "debug")
            if os.path.exists(target_dir):
                for root, dirs, files in os.walk(target_dir):
                    for file in files:
                        cleaned_name = file.replace(".exe", "")
                        if cleaned_name in ["baker-cli", "axicor-node", "weaver-daemon"]:
                            rust_built = True
                            break
                    if rust_built:
                        break
            rust_status = "compiled" if rust_built else "cleaned"
            
            # Git status
            git_status = "clean"
            git_detail = "clean"
            try:
                import subprocess
                git_flags = {}
                if os.name == 'nt':
                    git_flags["creationflags"] = subprocess.CREATE_NO_WINDOW
                res = subprocess.run(
                    ["git", "status", "-s"],
                    capture_output=True,
                    text=True,
                    cwd=workspace_root,
                    **git_flags
                )
                lines = [l.strip() for l in res.stdout.split("\n") if l.strip()]
                changed_count = len(lines)
                if changed_count > 0:
                    git_status = "dirty"
                    git_detail = f"dirty · {changed_count} files"
            except Exception:
                pass
                
            response = {
                "server": server_status,
                "rust": rust_status,
                "git": git_status,
                "git_detail": git_detail,
                "watch": "off"
            }
            
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(json.dumps(response).encode("utf-8"))
            
        except Exception as e:
            print(f"Error in handle_dev_status: {e}")
            self.send_response(500)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            response = {"status": "error", "message": str(e)}
            self.wfile.write(json.dumps(response).encode("utf-8"))

    def handle_dev_command(self):
        try:
            content_length = int(self.headers.get("Content-Length", 0))
            post_data = self.rfile.read(content_length)
            payload = json.loads(post_data.decode("utf-8"))
            
            command = payload.get("command")
            if not command:
                raise ValueError("Missing command")
                
            # List of allowed developer commands
            allowed_commands = ["start", "stop", "restart", "clean", "build", "test", "status", "git"]
            if command not in allowed_commands:
                raise ValueError(f"Command '{command}' is not supported or not allowed")
                
            import subprocess
            workspace_root = os.path.abspath(os.path.join(ROOT_DIR, ".."))
            
            cmd_args = [sys.executable, ".AxiConsole/execute.py", command]
            
            subprocess_flags = {}
            if os.name == 'nt':
                subprocess_flags["creationflags"] = subprocess.CREATE_NO_WINDOW
                
            res = subprocess.run(
                cmd_args,
                cwd=workspace_root,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="ignore",
                **subprocess_flags
            )
            
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            
            response = {
                "status": "success" if res.returncode == 0 else "error",
                "command": command,
                "stdout": res.stdout,
                "stderr": res.stderr,
                "exit_code": res.returncode
            }
            self.wfile.write(json.dumps(response).encode("utf-8"))
            
        except Exception as e:
            print(f"Error in handle_dev_command: {e}")
            self.send_response(500)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            response = {"status": "error", "message": str(e)}
            self.wfile.write(json.dumps(response).encode("utf-8"))

    def do_OPTIONS(self):
        # Support CORS for local development
        self.send_response(200)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.end_headers()


def run_server():
    server_address = ("", PORT)
    httpd = HTTPServer(server_address, VisualizerHandler)
    print(f"Visualizer Server running at http://localhost:{PORT}/")
    print(f"Serving files from: {STATIC_DIR}")
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nStopping server...")
        httpd.server_close()


if __name__ == "__main__":
    run_server()
