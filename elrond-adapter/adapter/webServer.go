package adapter

import (
	"errors"
	"net/http"

	models "github.com/ElrondNetwork/elrond-adapter/data"
	"github.com/gin-gonic/gin"
)

type webServer struct {
	router  *gin.Engine
	adapter *adapter
}

func NewWebServer(adapter *adapter) (*webServer, error) {
	if adapter == nil {
		return nil, errors.New("nil adapter provided")
	}

	return &webServer{
		router:  gin.Default(),
		adapter: adapter,
	}, nil
}

func (ws *webServer) Run(port string) {
	ws.router.POST("/write", ws.processWriteRequest)
	ws.router.POST("/price", ws.processPriceRequest)
	ws.router.POST("/job", ws.processJobRunRequest)

	if err := ws.router.Run(port); err != nil {
		panic(err)
	}
}

func (ws *webServer) processWriteRequest(c *gin.Context) {
	var req models.JobRequest
	if err := c.BindJSON(&req); err != nil {
		errResponse(c, http.StatusBadRequest)
		return
	}

	responseData, err := ws.adapter.HandleWriteFeed(req.Data)
	if err != nil {
		errResponse(c, http.StatusInternalServerError)
		return
	}

	okResponse(c, responseData, req.JobID)
}

func (ws *webServer) processPriceRequest(c *gin.Context) {
	var req models.JobRequest
	if err := c.BindJSON(&req); err != nil {
		errResponse(c, http.StatusBadRequest)
		return
	}

	responseData, err := ws.adapter.HandlePriceFeed(req.Data)
	if err != nil {
		errResponse(c, http.StatusInternalServerError)
		return
	}

	okResponse(c, responseData, req.JobID)
}

func (ws *webServer) processJobRunRequest(c *gin.Context) {
	var req models.JobRequest
	if err := c.BindJSON(&req); err != nil {
		errResponse(c, http.StatusBadRequest)
		return
	}

	responseData, err := ws.adapter.HandlePriceFeedJob()
	if err != nil {
		errResponse(c, http.StatusInternalServerError)
		return
	}

	result := map[string][]string{"txHashes": responseData}
	okResponse(c, result, req.JobID)
}

func okResponse(c *gin.Context, value interface{}, jobID string) {
	c.JSON(http.StatusOK, models.JobResponse{
		JobRunID:   jobID,
		Data:       gin.H{"result": value},
		Result:     value,
		StatusCode: http.StatusOK,
	})
}

func errResponse(c *gin.Context, errCode int) {
	c.JSON(errCode, models.JobResponse{
		JobRunID:   "",
		Data:       nil,
		StatusCode: errCode,
	})
}
