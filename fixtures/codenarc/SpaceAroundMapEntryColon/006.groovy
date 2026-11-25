
            devToolsProject.run(
              setup: { data ->
                data['venv'] = virtualenv.create('python3.8')
                data.venv.run('pip install -r requirements-dev.txt -r requirements.txt')
              },
              // etc...
            )
            