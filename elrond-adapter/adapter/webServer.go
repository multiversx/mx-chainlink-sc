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

func (ws *webServer) Run(port string) *http.Server {
	ws.router.POST("/write", ws.processWriteRequest)
	ws.router.POST("/price", ws.processPriceRequest)
	ws.router.POST("/price-job", ws.processJobRunRequest)
	ws.router.POST("/ethgas/denominate", ws.processGasRequest)

	server := &http.Server{
		Addr:    port,
		Handler: ws.router,
	}
	go func() {
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			panic(err)
		}
	}()
	return server
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

func (ws *webServer) processGasRequest(c *gin.Context) {
	var req models.JobRequest
	if err := c.BindJSON(&req); err != nil {
		errResponse(c, http.StatusBadRequest)
		return
	}

	gasValue, err := ws.adapter.HandleEthGasDenomination()
	if err != nil {
		errResponse(c, http.StatusInternalServerError)
		return
	}

	okResponse(c, gasValue, req.JobID)
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
