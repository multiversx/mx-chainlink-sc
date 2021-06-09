package main

import (
	"context"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"time"

	"github.com/ElrondNetwork/elrond-adapter/adapter"
	"github.com/ElrondNetwork/elrond-adapter/config"
	logger "github.com/ElrondNetwork/elrond-go-logger"
	"github.com/ElrondNetwork/elrond-go-logger/check"
	nodeFactory "github.com/ElrondNetwork/elrond-go/cmd/node/factory"
	"github.com/ElrondNetwork/elrond-go/core/logging"
	"github.com/urfave/cli"
)

const (
	defaultLogsPath    = "logs"
	logFilePrefix      = "elrond-adapter"
	logFileLifeSpanSec = 86400
)

var (
	cliHelpTemplate = `NAME:
   {{.Name}} - {{.Usage}}
USAGE:
   {{.HelpName}} {{if .VisibleFlags}}[global options]{{end}}
   {{if len .Authors}}
AUTHOR:
   {{range .Authors}}{{ . }}{{end}}
   {{end}}{{if .Commands}}
GLOBAL OPTIONS:
   {{range .VisibleFlags}}{{.}}
   {{end}}
VERSION:
   {{.Version}}
   {{end}}
`
	log = logger.GetOrCreate("elrond-adapter")

	logLevel = cli.StringFlag{
		Name:  "log-level",
		Usage: "This flag specifies the log level. Options: *:NONE | ERROR | WARN | INFO | DEBUG | TRACE",
		Value: fmt.Sprintf("*:%s", logger.LogInfo.String()),
	}

	logSaveFile = cli.BoolFlag{
		Name:  "log-save",
		Usage: "Boolean option for enabling log saving",
	}

	workingDirectory = cli.StringFlag{
		Name:  "working-directory",
		Usage: "his flag specifies the `directory` where the adapter will store logs.",
		Value: "",
	}
)

func main() {
	app := cli.NewApp()
	cli.AppHelpTemplate = cliHelpTemplate
	app.Name = "Elrond Chainlink Adapter"
	app.Flags = []cli.Flag{
		logLevel,
		logSaveFile,
		workingDirectory,
	}
	app.Authors = []cli.Author{
		{
			Name:  "The Elrond Team",
			Email: "contact@elrond.com",
		},
	}
	app.Action = startAdapter
	err := app.Run(os.Args)
	if err != nil {
		log.Error(err.Error())
		panic(err)
	}
}

func startAdapter(ctx *cli.Context) error {
	log.Info("starting adapter...")

	fileLogging, err := initLogger(ctx)
	if err != nil {
		return err
	}
	cfg, err := config.LoadConfig()
	if err != nil {
		return err
	}

	adapterFacade, err := adapter.NewAdapter(cfg)
	if err != nil {
		return err
	}

	webServer, err := adapter.NewWebServer(adapterFacade)
	if err != nil {
		return err
	}
	server := webServer.Run(cfg.Server.Port)

	waitForGracefulShutdown(server)

	log.Debug("closing adapter")
	if !check.IfNil(fileLogging) {
		err = fileLogging.Close()
		if err != nil {
			return err
		}
	}
	return nil
}

func initLogger(ctx *cli.Context) (nodeFactory.FileLoggingHandler, error) {
	logLevelValue := ctx.GlobalString(logLevel.Name)
	err := logger.SetLogLevel(logLevelValue)
	if err != nil {
		return nil, err
	}
	workingDir, err := getWorkingDir(ctx)
	if err != nil {
		return nil, err
	}
	var fileLogging nodeFactory.FileLoggingHandler
	saveLogs := ctx.GlobalBool(logSaveFile.Name)
	if saveLogs {
		fileLogging, err = logging.NewFileLogging(workingDir, defaultLogsPath, logFilePrefix)
		if err != nil {
			return fileLogging, err
		}
	}
	if !check.IfNil(fileLogging) {
		err = fileLogging.ChangeFileLifeSpan(time.Second * time.Duration(logFileLifeSpanSec))
		if err != nil {
			return nil, err
		}
	}

	return fileLogging, nil
}

func waitForGracefulShutdown(server *http.Server) {
	quit := make(chan os.Signal)
	signal.Notify(quit, os.Interrupt, os.Kill)
	<-quit

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := server.Shutdown(ctx); err != nil {
		log.Error("error shutting down server", "error", err.Error())
		panic(err)
	}
	_ = server.Close()
}

func getWorkingDir(ctx *cli.Context) (string, error) {
	if ctx.IsSet(workingDirectory.Name) {
		return ctx.GlobalString(workingDirectory.Name), nil
	}
	return os.Getwd()
}
